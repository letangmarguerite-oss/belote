//! Salons de jeu : creation, consultation, arrivee d'un joueur.
//!
//! Une table a toujours quatre sieges. A la creation, le proprietaire prend le
//! siege 0 et les trois autres sont tenus par des bots ; un ami qui rejoint
//! remplace un bot.

use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::AppState;

/// Alphabet sans caracteres ambigus (ni 0/O, ni 1/I/L) : le code se dicte au
/// telephone sans se tromper.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LENGTH: usize = 6;

#[derive(Serialize)]
pub struct TableResponse {
    pub id: Uuid,
    pub join_code: String,
    pub status: String,
    pub owner_id: Uuid,
    pub seats: Vec<SeatResponse>,
}

#[derive(Serialize)]
pub struct SeatResponse {
    pub seat: i16,
    pub user_id: Option<Uuid>,
    pub display_name: Option<String>,
    pub is_bot: bool,
}

#[derive(sqlx::FromRow)]
struct TableRow {
    id: Uuid,
    join_code: String,
    status: String,
    owner_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct SeatRow {
    seat: i16,
    user_id: Option<Uuid>,
    display_name: Option<String>,
    is_bot: bool,
}

// ---------------------------------------------------------------------------

pub async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<TableResponse>> {
    let table_id = Uuid::now_v7();

    // Le code est court : une collision est possible, on reessaie.
    let mut join_code = String::new();
    for attempt in 0..8 {
        let candidate = generate_join_code();
        let inserted = sqlx::query(
            "insert into game_tables (id, join_code, owner_id)
             values ($1, $2, $3)
             on conflict (join_code) do nothing",
        )
        .bind(table_id)
        .bind(&candidate)
        .bind(user_id)
        .execute(&state.pool)
        .await?;

        if inserted.rows_affected() == 1 {
            join_code = candidate;
            break;
        }
        if attempt == 7 {
            return Err(ApiError::Internal(anyhow::anyhow!(
                "impossible de generer un code de salon libre"
            )));
        }
    }

    // Le createur s'assoit au sud, les trois autres sieges sont des bots.
    for seat in 0..4i16 {
        let is_owner = seat == 0;
        sqlx::query(
            "insert into table_seats (table_id, seat, user_id, is_bot) values ($1, $2, $3, $4)",
        )
        .bind(table_id)
        .bind(seat)
        .bind(if is_owner { Some(user_id) } else { None })
        .bind(!is_owner)
        .execute(&state.pool)
        .await?;
    }

    load_table(&state, &join_code).await.map(Json)
}

pub async fn show(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(code): Path<String>,
) -> ApiResult<Json<TableResponse>> {
    load_table(&state, &code.to_uppercase()).await.map(Json)
}

/// Prend le premier siege libre. Si le joueur est deja assis, l'appel est
/// idempotent : il retrouve sa place (cas du rechargement de page).
pub async fn join(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(code): Path<String>,
) -> ApiResult<Json<TableResponse>> {
    let code = code.to_uppercase();
    let table: TableRow = sqlx::query_as(
        "select id, join_code, status, owner_id from game_tables where join_code = $1",
    )
    .bind(&code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    let already: Option<(i16,)> =
        sqlx::query_as("select seat from table_seats where table_id = $1 and user_id = $2")
            .bind(table.id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;

    if already.is_none() {
        // `returning` nous dit si un siege a effectivement ete pris : la clause
        // `is_bot` empeche deux joueurs de se disputer la meme place.
        let taken: Option<(i16,)> = sqlx::query_as(
            "update table_seats
                set user_id = $2, is_bot = false
              where table_id = $1
                and seat = (
                    select seat from table_seats
                     where table_id = $1 and user_id is null
                     order by seat
                     limit 1
                     for update
                )
          returning seat",
        )
        .bind(table.id)
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await?;

        if taken.is_none() {
            return Err(ApiError::Conflict("cette table est complete".into()));
        }
    }

    load_table(&state, &code).await.map(Json)
}

// ---------------------------------------------------------------------------

async fn load_table(state: &AppState, code: &str) -> ApiResult<TableResponse> {
    let table: TableRow = sqlx::query_as(
        "select id, join_code, status, owner_id from game_tables where join_code = $1",
    )
    .bind(code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    let seats: Vec<SeatRow> = sqlx::query_as(
        "select s.seat, s.user_id, u.display_name, s.is_bot
           from table_seats s
           left join users u on u.id = s.user_id
          where s.table_id = $1
          order by s.seat",
    )
    .bind(table.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(TableResponse {
        id: table.id,
        join_code: table.join_code,
        status: table.status,
        owner_id: table.owner_id,
        seats: seats
            .into_iter()
            .map(|s| SeatResponse {
                seat: s.seat,
                user_id: s.user_id,
                display_name: s.display_name,
                is_bot: s.is_bot,
            })
            .collect(),
    })
}

fn generate_join_code() -> String {
    let mut rng = rand::thread_rng();
    (0..CODE_LENGTH)
        .map(|_| CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn join_codes_avoid_ambiguous_characters() {
        let code = generate_join_code();
        assert_eq!(code.len(), CODE_LENGTH);
        for ch in code.chars() {
            assert!(
                !"O0I1L".contains(ch),
                "{ch} se confond avec un autre caractere"
            );
        }
    }

    #[test]
    fn join_codes_are_not_obviously_repetitive() {
        let codes: HashSet<String> = (0..200).map(|_| generate_join_code()).collect();
        assert!(codes.len() > 190, "trop de collisions sur 200 tirages");
    }
}
