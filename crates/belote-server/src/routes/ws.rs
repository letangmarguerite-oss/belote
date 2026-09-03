//! Connexion temps reel.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{ApiError, ApiResult};
use crate::proto::{ClientMsg, ServerMsg};
use crate::table::Cmd;
use crate::tickets::TICKET_TTL;
use crate::AppState;

/// Tampon d'envoi par connexion. Genereux : mieux vaut de la memoire qu'un
/// message perdu.
const OUT_BUFFER: usize = 128;

static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize)]
pub struct TicketResponse {
    pub ticket: String,
    pub expires_in: u64,
}

/// Le client obtient ici le ticket qu'il presentera a l'ouverture du socket.
pub async fn issue_ticket(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<TicketResponse>> {
    Ok(Json(TicketResponse {
        ticket: state.tickets.issue(user_id),
        expires_in: TICKET_TTL.as_secs(),
    }))
}

#[derive(Deserialize)]
pub struct WsParams {
    pub ticket: String,
    pub code: String,
}

pub async fn connect(
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
    ws: WebSocketUpgrade,
) -> ApiResult<Response> {
    let user_id = state
        .tickets
        .redeem(&params.ticket)
        .ok_or(ApiError::Unauthorized)?;

    let code = params.code.to_uppercase();

    // On refuse l'ouverture plutot que de laisser l'acteur repondre : le client
    // recoit un vrai statut HTTP, plus facile a diagnostiquer.
    let seated: Option<(i16,)> = sqlx::query_as(
        "select s.seat
           from table_seats s
           join game_tables t on t.id = s.table_id
          where t.join_code = $1 and s.user_id = $2",
    )
    .bind(&code)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    if seated.is_none() {
        return Err(ApiError::Forbidden);
    }

    let cmd_tx = state
        .tables
        .get_or_spawn(&state.pool, &state.persist, &code)
        .await?;

    Ok(ws.on_upgrade(move |socket| serve(socket, cmd_tx, user_id)))
}

async fn serve(socket: WebSocket, cmd_tx: mpsc::Sender<Cmd>, user_id: Uuid) {
    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
    let (mut sink, mut stream) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<ServerMsg>(OUT_BUFFER);

    // Le canal de reponse directe (Pong) doit survivre au deplacement dans Connect.
    let pong_tx = out_tx.clone();

    if cmd_tx
        .send(Cmd::Connect {
            user_id,
            conn_id,
            tx: out_tx,
        })
        .await
        .is_err()
    {
        return;
    }

    let mut writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let Ok(text) = serde_json::to_string(&msg) else {
                continue;
            };
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    let reader_cmd = cmd_tx.clone();
    let mut reader = tokio::spawn(async move {
        while let Some(Ok(message)) = stream.next().await {
            match message {
                Message::Text(text) => match serde_json::from_str::<ClientMsg>(&text) {
                    Ok(ClientMsg::Act { action }) => {
                        if reader_cmd.send(Cmd::Act { user_id, action }).await.is_err() {
                            break;
                        }
                    }
                    Ok(ClientMsg::Ready) => {
                        if reader_cmd.send(Cmd::Ready { user_id }).await.is_err() {
                            break;
                        }
                    }
                    Ok(ClientMsg::Start) => {
                        if reader_cmd.send(Cmd::Start { user_id }).await.is_err() {
                            break;
                        }
                    }
                    Ok(ClientMsg::Say { phrase }) => {
                        if reader_cmd
                            .send(Cmd::Say { user_id, phrase })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(ClientMsg::Resync) => {
                        if reader_cmd.send(Cmd::Resync { conn_id }).await.is_err() {
                            break;
                        }
                    }
                    Ok(ClientMsg::Ping) => {
                        let _ = pong_tx.try_send(ServerMsg::Pong);
                    }
                    Err(err) => {
                        tracing::debug!(%err, "message client illisible");
                        let _ = pong_tx.try_send(ServerMsg::Error {
                            message: "message illisible".into(),
                        });
                    }
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // La premiere des deux taches qui s'arrete met fin a la connexion.
    tokio::select! {
        _ = &mut writer => reader.abort(),
        _ = &mut reader => writer.abort(),
    }

    let _ = cmd_tx.send(Cmd::Disconnect { conn_id }).await;
}
