//! Statistiques d'un joueur.
//!
//! Rien n'est compte a l'avance : tout se deduit du journal, en interrogeant
//! les evenements `scored` des parties ou le joueur figure. Une table de
//! compteurs a maintenir en parallele finirait par diverger du journal, qui
//! reste la seule source de verite.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Stats {
    /// Parties menees a leur terme, victoire ou defaite.
    pub games_finished: i64,
    pub games_won: i64,
    /// Donnes jouees, toutes parties confondues.
    pub deals_played: i64,
    /// Donnes ou ce joueur a pris.
    pub deals_taken: i64,
    /// Parmi celles-la, contrats reussis.
    pub deals_taken_made: i64,
    /// Meilleur score de son camp sur une seule donne.
    pub best_deal: i64,
    /// Belotes annoncees par ce joueur.
    pub belotes: i64,
    /// Capots realises par son camp.
    pub capots: i64,
}

pub async fn show(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<Stats>> {
    // `p.seat` donne la place du joueur dans chaque partie ; son equipe est la
    // parite de ce siege. Les evenements `scored` portent le preneur, le
    // detenteur de la belote, le capot et les points de chaque camp.
    let stats: Stats = sqlx::query_as(
        r#"
        with mine as (
            select g.id,
                   g.ended_at,
                   g.final_scores,
                   p.seat,
                   (p.seat % 2) as team
              from games g
              join game_players p on p.game_id = g.id
             where p.user_id = $1
        ),
        deals as (
            select m.team,
                   m.seat,
                   e.payload as s
              from mine m
              join game_events e on e.game_id = m.id
             where e.payload->>'type' = 'scored'
        )
        select
            (select count(*) from mine
              where ended_at is not null
                and (final_scores->>'completed')::boolean is true)          as games_finished,
            (select count(*) from mine
              where ended_at is not null
                and (final_scores->>'completed')::boolean is true
                and (final_scores->'totals'->>team::int)::int
                  > (final_scores->'totals'->>(1 - team)::int)::int)          as games_won,
            (select count(*) from deals)                                      as deals_played,
            (select count(*) from deals
              where (s->>'taker')::int = seat)                                as deals_taken,
            (select count(*) from deals
              where (s->>'taker')::int = seat
                and (s->>'contract_made')::boolean is true)                   as deals_taken_made,
            -- `max` d'un int reste un int4 : on l'aligne sur les compteurs.
            (select coalesce(max((s->'points'->>team::int)::int), 0)::bigint
               from deals)                                                    as best_deal,
            (select count(*) from deals
              where s->>'belote' is not null
                and (s->>'belote')::int = seat)                               as belotes,
            (select count(*) from deals
              where s->>'capot' is not null
                and (s->>'capot')::int = team)                                as capots
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(stats))
}
