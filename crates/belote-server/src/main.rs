//! Serveur de belote.

mod auth;
mod config;
mod error;
mod proto;
mod routes;
mod table;
mod tickets;

use std::sync::Arc;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::table::{PersistMsg, TableRegistry};
use crate::tickets::TicketStore;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    /// Les tables vivantes, une tache tokio chacune.
    pub tables: TableRegistry,
    /// Tickets a usage unique pour ouvrir un WebSocket.
    pub tickets: TicketStore,
    /// Ecriture du journal, hors du chemin critique du jeu.
    pub persist: mpsc::UnboundedSender<PersistMsg>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("belote_server=debug,tower_http=debug,info")),
        )
        .init();

    let config = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connexion a Postgres impossible")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("les migrations ont echoue")?;
    tracing::info!("migrations a jour");

    // Une partie ouverte sans tache pour la porter ne reprendra jamais : elle
    // date d'un arret precedent. On la clot, sans quoi elle resterait affichee
    // "en cours" indefiniment dans l'historique des joueurs.
    let orphans = sqlx::query(
        r#"update games
              set ended_at = now(),
                  final_scores = coalesce(final_scores, '{}'::jsonb)
                                 || '{"completed": false}'::jsonb
            where ended_at is null"#,
    )
    .execute(&pool)
    .await
    .context("cloture des parties orphelines impossible")?;

    if orphans.rows_affected() > 0 {
        tracing::info!(
            parties = orphans.rows_affected(),
            "parties laissees ouvertes par un arret precedent, cloturees"
        );
    }

    let port = config.port;
    let persist = table::spawn_persister(pool.clone());
    let state = AppState {
        pool,
        config: Arc::new(config),
        tables: TableRegistry::default(),
        tickets: TicketStore::default(),
        persist,
    };

    let app = routes::router(state);
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("serveur a l'ecoute sur http://0.0.0.0:{port}");

    axum::serve(listener, app).await?;
    Ok(())
}
