//! Comptes : inscription, connexion, rotation du jeton, deconnexion.

use axum::extract::State;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::auth::{
    self, AuthUser, ACCESS_TTL_SECONDS, REFRESH_COOKIE, REFRESH_TTL_DAYS,
};
use crate::error::{ApiError, ApiResult};
use crate::AppState;

#[derive(Deserialize)]
pub struct RegisterBody {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    display_name: String,
    password_hash: String,
}

// ---------------------------------------------------------------------------

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterBody>,
) -> ApiResult<(CookieJar, Json<AuthResponse>)> {
    let email = normalize_email(&body.email)?;
    let display_name = validate_display_name(&body.display_name)?;
    validate_password(&body.password)?;

    let password_hash = auth::hash_password(&body.password).map_err(ApiError::Internal)?;
    let user_id = Uuid::now_v7();

    let inserted = sqlx::query(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, $3, $4)
         on conflict do nothing",
    )
    .bind(user_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(&display_name)
    .execute(&state.pool)
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(ApiError::Conflict("cette adresse est deja utilisee".into()));
    }

    let jar = issue_session(&state, jar, user_id).await?;
    let token = auth::issue_access_token(user_id, &state.config.jwt_secret)
        .map_err(ApiError::Internal)?;

    Ok((
        jar,
        Json(AuthResponse {
            access_token: token,
            expires_in: ACCESS_TTL_SECONDS,
            user: UserResponse {
                id: user_id,
                email,
                display_name,
            },
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> ApiResult<(CookieJar, Json<AuthResponse>)> {
    let email = normalize_email(&body.email)?;

    let user: Option<UserRow> = sqlx::query_as(
        "select id, email, display_name, password_hash from users where lower(email) = lower($1)",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;

    // Meme reponse que le compte existe ou non : on ne revele pas les comptes.
    let Some(user) = user else {
        // On hache quand meme pour ne pas trahir l'absence par le temps de reponse.
        let _ = auth::hash_password(&body.password);
        return Err(ApiError::Unauthorized);
    };

    if !auth::verify_password(&body.password, &user.password_hash) {
        return Err(ApiError::Unauthorized);
    }

    let jar = issue_session(&state, jar, user.id).await?;
    let token =
        auth::issue_access_token(user.id, &state.config.jwt_secret).map_err(ApiError::Internal)?;

    Ok((
        jar,
        Json(AuthResponse {
            access_token: token,
            expires_in: ACCESS_TTL_SECONDS,
            user: UserResponse {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
            },
        }),
    ))
}

/// Echange le cookie de rafraichissement contre un nouveau jeton d'acces.
/// Le jeton de rafraichissement est tourne a chaque appel : un jeton vole ne
/// sert qu'une fois, et son usage invalide la session legitime.
pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<AuthResponse>)> {
    let cookie = jar.get(REFRESH_COOKIE).ok_or(ApiError::Unauthorized)?;
    let hash = auth::hash_refresh_token(cookie.value());

    // On revoque et on lit l'utilisateur en une seule requete atomique.
    let user_id: Option<(Uuid,)> = sqlx::query_as(
        "update refresh_tokens
            set revoked_at = now()
          where token_hash = $1
            and revoked_at is null
            and expires_at > now()
      returning user_id",
    )
    .bind(&hash)
    .fetch_optional(&state.pool)
    .await?;

    let Some((user_id,)) = user_id else {
        return Err(ApiError::Unauthorized);
    };

    let user: UserRow = sqlx::query_as(
        "select id, email, display_name, password_hash from users where id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    let jar = issue_session(&state, jar, user_id).await?;
    let token =
        auth::issue_access_token(user_id, &state.config.jwt_secret).map_err(ApiError::Internal)?;

    Ok((
        jar,
        Json(AuthResponse {
            access_token: token,
            expires_in: ACCESS_TTL_SECONDS,
            user: UserResponse {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
            },
        }),
    ))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> ApiResult<CookieJar> {
    if let Some(cookie) = jar.get(REFRESH_COOKIE) {
        let hash = auth::hash_refresh_token(cookie.value());
        sqlx::query("update refresh_tokens set revoked_at = now() where token_hash = $1")
            .bind(&hash)
            .execute(&state.pool)
            .await?;
    }

    let mut removal = base_cookie(&state, REFRESH_COOKIE, String::new());
    removal.make_removal();
    Ok(jar.add(removal))
}

pub async fn me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<UserResponse>> {
    let user: UserResponse =
        sqlx::query_as("select id, email, display_name from users where id = $1")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await?;
    Ok(Json(user))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Cree un jeton de rafraichissement et le pose dans le cookie.
async fn issue_session(state: &AppState, jar: CookieJar, user_id: Uuid) -> ApiResult<CookieJar> {
    let (token, hash) = auth::generate_refresh_token();
    let expires_at = OffsetDateTime::now_utc() + Duration::days(REFRESH_TTL_DAYS);

    sqlx::query(
        "insert into refresh_tokens (id, user_id, token_hash, expires_at)
         values ($1, $2, $3, $4)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(&hash)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    let mut cookie = base_cookie(state, crate::auth::REFRESH_COOKIE, token);
    cookie.set_expires(expires_at);
    Ok(jar.add(cookie))
}

fn base_cookie(state: &AppState, name: &'static str, value: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, value);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_secure(state.config.secure_cookies);

    // Avec un domaine parent commun au front et a l'API, `Lax` suffit et evite
    // les blocages de cookies tiers (Safari, bloqueurs). Sans domaine partage,
    // il faut `None`, qui exige `Secure`.
    match &state.config.cookie_domain {
        Some(domain) => {
            cookie.set_domain(domain.clone());
            cookie.set_same_site(SameSite::Lax);
        }
        None if state.config.secure_cookies => cookie.set_same_site(SameSite::None),
        None => cookie.set_same_site(SameSite::Lax),
    }
    cookie
}

fn normalize_email(raw: &str) -> ApiResult<String> {
    let email = raw.trim().to_lowercase();
    if email.len() < 3 || !email.contains('@') || email.len() > 254 {
        return Err(ApiError::BadRequest("adresse email invalide".into()));
    }
    Ok(email)
}

fn validate_password(password: &str) -> ApiResult<()> {
    if password.chars().count() < 8 {
        return Err(ApiError::BadRequest(
            "le mot de passe doit faire au moins 8 caracteres".into(),
        ));
    }
    if password.len() > 512 {
        return Err(ApiError::BadRequest("mot de passe trop long".into()));
    }
    Ok(())
}

fn validate_display_name(raw: &str) -> ApiResult<String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 32 {
        return Err(ApiError::BadRequest(
            "le pseudo doit faire entre 1 et 32 caracteres".into(),
        ));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_are_normalized_and_checked() {
        assert_eq!(normalize_email("  Jean@Exemple.FR ").unwrap(), "jean@exemple.fr");
        assert!(normalize_email("pas-une-adresse").is_err());
        assert!(normalize_email("a@").is_err() || normalize_email("a@").is_ok());
    }

    #[test]
    fn short_passwords_are_refused() {
        assert!(validate_password("court").is_err());
        assert!(validate_password("assez-long").is_ok());
    }

    #[test]
    fn display_names_are_trimmed_and_bounded() {
        assert_eq!(validate_display_name("  Leo  ").unwrap(), "Leo");
        assert!(validate_display_name("   ").is_err());
        assert!(validate_display_name(&"x".repeat(33)).is_err());
    }
}
