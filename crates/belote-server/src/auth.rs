//! Authentification : mots de passe, jetons d'acces, jetons de rafraichissement.
//!
//! - Mot de passe : Argon2id, sel aleatoire par utilisateur (gere par la crate).
//! - Acces : JWT court (15 min), garde en memoire par le client.
//! - Rafraichissement : jeton opaque aleatoire, stocke *hache* en base et pose
//!   dans un cookie HttpOnly, avec rotation a chaque usage.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

/// Duree de vie du jeton d'acces.
pub const ACCESS_TTL_SECONDS: i64 = 15 * 60;
/// Duree de vie du jeton de rafraichissement.
pub const REFRESH_TTL_DAYS: i64 = 30;
/// Nom du cookie portant le jeton de rafraichissement.
pub const REFRESH_COOKIE: &str = "belote_refresh";

// ---------------------------------------------------------------------------
// Mots de passe
// ---------------------------------------------------------------------------

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("hachage du mot de passe: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Jeton d'acces
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_access_token(user_id: Uuid, secret: &str) -> anyhow::Result<String> {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let claims = Claims {
        sub: user_id,
        iat: now,
        exp: now + ACCESS_TTL_SECONDS,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(Into::into)
}

pub fn decode_access_token(token: &str, secret: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized)
}

/// Extracteur : refuse la requete si l'en-tete `Authorization: Bearer` manque
/// ou si le jeton est invalide ou expire.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;

        let claims = decode_access_token(token, &state.config.jwt_secret)?;
        Ok(AuthUser(claims.sub))
    }
}

// ---------------------------------------------------------------------------
// Jeton de rafraichissement
// ---------------------------------------------------------------------------

/// Un jeton aleatoire de 32 octets. On renvoie la forme lisible (pour le
/// cookie) et son empreinte (seule chose stockee en base).
pub fn generate_refresh_token() -> (String, Vec<u8>) {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash_refresh_token(&token);
    (token, hash)
}

/// SHA-256 suffit ici : le jeton est deja aleatoire sur 256 bits, il n'y a rien
/// a deviner par force brute (contrairement a un mot de passe).
pub fn hash_refresh_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_verifies_against_its_own_hash_only() {
        let hash = hash_password("belote1234").unwrap();
        assert!(verify_password("belote1234", &hash));
        assert!(!verify_password("belote1235", &hash));
        assert!(!verify_password("", &hash));
    }

    #[test]
    fn two_identical_passwords_get_different_hashes() {
        let a = hash_password("meme-mot-de-passe").unwrap();
        let b = hash_password("meme-mot-de-passe").unwrap();
        assert_ne!(a, b, "le sel doit differer d'un utilisateur a l'autre");
    }

    #[test]
    fn a_malformed_hash_never_validates() {
        assert!(!verify_password("x", "pas-un-hash-argon2"));
    }

    #[test]
    fn an_access_token_round_trips() {
        let secret = "un-secret-de-test-suffisamment-long!!";
        let id = Uuid::new_v4();
        let token = issue_access_token(id, secret).unwrap();
        assert_eq!(decode_access_token(&token, secret).unwrap().sub, id);
    }

    #[test]
    fn a_token_signed_with_another_secret_is_rejected() {
        let token = issue_access_token(Uuid::new_v4(), "un-secret-de-test-suffisamment-long!!").unwrap();
        assert!(decode_access_token(&token, "un-autre-secret-tout-aussi-long!!!!!").is_err());
    }

    #[test]
    fn refresh_tokens_are_unique_and_stored_hashed() {
        let (token_a, hash_a) = generate_refresh_token();
        let (token_b, _) = generate_refresh_token();
        assert_ne!(token_a, token_b);
        assert_eq!(hash_a, hash_refresh_token(&token_a));
        assert_ne!(hash_a, token_a.as_bytes(), "le clair ne doit pas etre stocke");
    }
}
