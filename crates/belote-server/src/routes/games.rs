//! Historique des parties.
//!
//! Une partie ne se stocke pas comme un etat : elle se stocke comme la suite
//! de ses evenements. Revoir une partie, c'est rejouer ce journal.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::AppState;

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

#[derive(Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GameSummary {
    pub id: Uuid,
    pub table_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    pub final_scores: Option<serde_json::Value>,
    pub seat: i16,
}

#[derive(Serialize)]
pub struct GameDetail {
    pub id: Uuid,
    pub table_id: Uuid,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    pub final_scores: Option<serde_json::Value>,
    pub players: Vec<GamePlayer>,
    /// Le journal complet, dans l'ordre. A rejouer avec `belote_core::rules::reduce`.
    pub events: Vec<serde_json::Value>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GamePlayer {
    pub seat: i16,
    pub user_id: Option<Uuid>,
    pub display_name: Option<String>,
    pub is_bot: bool,
}

/// Les parties auxquelles le joueur a participe, les plus recentes d'abord.
pub async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Vec<GameSummary>>> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let games: Vec<GameSummary> = sqlx::query_as(
        "select g.id, g.table_id, g.started_at, g.ended_at, g.final_scores, p.seat
           from games g
           join game_players p on p.game_id = g.id
          where p.user_id = $1
          order by g.started_at desc
          limit $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(games))
}

/// Le detail d'une partie, journal compris. Reserve a ses participants.
pub async fn show(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(game_id): Path<Uuid>,
) -> ApiResult<Json<GameDetail>> {
    let participated: Option<(i16,)> =
        sqlx::query_as("select seat from game_players where game_id = $1 and user_id = $2")
            .bind(game_id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;

    // On ne distingue pas "partie inexistante" de "partie d'autrui" : cela
    // eviterait de confirmer l'existence d'un identifiant devine.
    if participated.is_none() {
        return Err(ApiError::NotFound);
    }

    let game: (Uuid, Uuid, OffsetDateTime, Option<OffsetDateTime>, Option<serde_json::Value>) =
        sqlx::query_as(
            "select id, table_id, started_at, ended_at, final_scores from games where id = $1",
        )
        .bind(game_id)
        .fetch_one(&state.pool)
        .await?;

    let players: Vec<GamePlayer> = sqlx::query_as(
        "select p.seat, p.user_id, u.display_name, p.is_bot
           from game_players p
           left join users u on u.id = p.user_id
          where p.game_id = $1
          order by p.seat",
    )
    .bind(game_id)
    .fetch_all(&state.pool)
    .await?;

    let events: Vec<(serde_json::Value,)> =
        sqlx::query_as("select payload from game_events where game_id = $1 order by seq")
            .bind(game_id)
            .fetch_all(&state.pool)
            .await?;

    Ok(Json(GameDetail {
        id: game.0,
        table_id: game.1,
        started_at: game.2,
        ended_at: game.3,
        final_scores: game.4,
        players,
        events: events.into_iter().map(|(p,)| p).collect(),
    }))
}
