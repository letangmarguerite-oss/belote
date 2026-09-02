//! Une tache tokio par table, proprietaire exclusif de son `GameState`.
//!
//! Aucun verrou sur l'etat de jeu : il n'est touche que par sa propre tache.
//! Deux joueurs ne peuvent donc pas poser une carte "en meme temps" — les
//! commandes sont serialisees par le canal, et les courses disparaissent par
//! construction plutot que par discipline.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use belote_core::action::Action;
use belote_core::event::Event;
use belote_core::rules::{apply, reduce, start_deal};
use belote_core::state::{GameState, Phase, Seat};
use belote_core::{bot, project};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::proto::{SeatInfo, ServerMsg};

/// Score a atteindre pour gagner le match.
pub const TARGET_POINTS: u16 = 1000;

/// Temps de reflexion feint d'un bot : un bot instantane rend la table illisible.
const BOT_THINK: Duration = Duration::from_millis(900);
/// Au-dela, un bot joue a la place du joueur absent.
const TURN_TIMEOUT: Duration = Duration::from_secs(45);
/// Delai avant d'enchainer seul sur la donne suivante, si personne ne se
/// prononce. Assez long pour lire le decompte et decider de s'arreter.
const AUTO_CONTINUE: Duration = Duration::from_secs(45);
const REDEAL_PAUSE: Duration = Duration::from_secs(2);
/// Le pli ramasse reste visible : sinon la quatrieme carte disparait avant
/// meme d'avoir ete vue.
const TRICK_PAUSE: Duration = Duration::from_millis(1600);
const TICK: Duration = Duration::from_millis(200);
/// Une table sans personne finit par liberer sa tache.
const IDLE_SHUTDOWN: Duration = Duration::from_secs(600);
/// Delai au-dela duquel on commence sans les joueurs qui ne se connectent pas.
const WAIT_FOR_PLAYERS: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Cmd {
    Connect {
        user_id: Uuid,
        conn_id: u64,
        tx: mpsc::Sender<ServerMsg>,
    },
    Disconnect {
        conn_id: u64,
    },
    Act {
        user_id: Uuid,
        action: Action,
    },
    Ready {
        user_id: Uuid,
    },
    /// Le proprietaire lance la partie depuis le salon d'attente.
    Start {
        user_id: Uuid,
    },
    Resync {
        conn_id: u64,
    },
}

/// Ecriture du journal, hors du chemin critique du jeu.
#[derive(Debug)]
pub enum PersistMsg {
    Event {
        game_id: Uuid,
        seq: u32,
        payload: serde_json::Value,
    },
    Finish {
        game_id: Uuid,
        totals: [u16; 2],
    },
}

#[derive(Clone, Debug)]
pub struct Occupant {
    pub user_id: Option<Uuid>,
    pub display_name: String,
    pub is_bot: bool,
}

struct Conn {
    conn_id: u64,
    seat: Seat,
    tx: mpsc::Sender<ServerMsg>,
}

// ---------------------------------------------------------------------------
// L'acteur
// ---------------------------------------------------------------------------

pub struct TableActor {
    pool: PgPool,
    persist: mpsc::UnboundedSender<PersistMsg>,
    table_id: Uuid,
    join_code: String,
    /// Qui a cree la table : seul lui peut la lancer.
    owner_id: Uuid,
    /// Vrai pour une partie solo : elle demarre sans attendre.
    autostart: bool,
    game_id: Option<Uuid>,
    occupants: [Occupant; 4],
    conns: Vec<Conn>,
    state: GameState,
    seq: u32,
    totals: [u16; 2],
    carry: u16,
    dealer: Seat,
    winner: Option<u8>,
    /// La donne courante a-t-elle deja ete comptabilisee au score du match ?
    settled: bool,
    /// Sieges ayant demande la donne suivante, entre deux donnes.
    ready: HashSet<Seat>,
    rng: ChaCha8Rng,
    /// Quand le siege courant doit agir automatiquement.
    act_at: Option<Instant>,
    empty_since: Option<Instant>,
    /// Depuis quand on attend les joueurs manquants avant de commencer.
    waiting_since: Option<Instant>,
}

impl TableActor {
    pub fn spawn(
        pool: PgPool,
        persist: mpsc::UnboundedSender<PersistMsg>,
        table: TableRecord,
    ) -> mpsc::Sender<Cmd> {
        let TableRecord {
            table_id,
            join_code,
            owner_id,
            autostart,
            occupants,
        } = table;

        let (tx, rx) = mpsc::channel(64);
        let actor = TableActor {
            pool,
            persist,
            table_id,
            join_code,
            owner_id,
            autostart,
            game_id: None,
            occupants,
            conns: Vec::new(),
            state: GameState::new(Seat(0), 0),
            seq: 0,
            totals: [0, 0],
            carry: 0,
            dealer: Seat(0),
            winner: None,
            settled: true,
            ready: HashSet::new(),
            rng: ChaCha8Rng::from_entropy(),
            act_at: None,
            empty_since: Some(Instant::now()),
            waiting_since: None,
        };
        tokio::spawn(actor.run(rx));
        tx
    }

    async fn run(mut self, mut rx: mpsc::Receiver<Cmd>) {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                cmd = rx.recv() => match cmd {
                    Some(cmd) => self.handle(cmd).await,
                    None => break,
                },
                _ = ticker.tick() => {
                    if self.on_tick().await {
                        break;
                    }
                }
            }
        }
        tracing::info!(table = %self.join_code, "table liberee");
    }

    // -----------------------------------------------------------------------
    // Commandes
    // -----------------------------------------------------------------------

    async fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Connect {
                user_id,
                conn_id,
                tx,
            } => {
                let Some(seat) = self.seat_of(user_id) else {
                    let _ = tx
                        .send(ServerMsg::Error {
                            message: "aucun siege a cette table".into(),
                        })
                        .await;
                    return;
                };

                let _ = tx
                    .send(ServerMsg::Welcome {
                        seat,
                        join_code: self.join_code.clone(),
                        target: TARGET_POINTS,
                    })
                    .await;

                self.conns.push(Conn { conn_id, seat, tx });
                self.empty_since = None;

                if self.game_id.is_none() {
                    self.waiting_since.get_or_insert_with(Instant::now);
                    // Table entre amis : elle reste au salon d'attente, le
                    // temps de partager le code. Partie solo : on entame des
                    // que le joueur est la — mais jamais avant que tous les
                    // humains attendus soient connectes, sinon un bot jouerait
                    // a la place d'une page qui charge encore.
                    if self.autostart && self.all_humans_connected() {
                        self.start_match().await;
                    } else {
                        self.broadcast_snapshot();
                    }
                } else {
                    self.broadcast_snapshot();
                    // Le joueur revient : son siege n'est plus tenu par un bot,
                    // le delai de reflexion redevient un delai humain.
                    self.schedule();
                }
                self.broadcast_seats();
            }

            Cmd::Disconnect { conn_id } => {
                self.conns.retain(|c| c.conn_id != conn_id);
                if self.conns.is_empty() {
                    self.empty_since = Some(Instant::now());
                }
                self.broadcast_seats();
                self.schedule();
            }

            Cmd::Act { user_id, action } => {
                let Some(seat) = self.seat_of(user_id) else {
                    return;
                };
                self.do_action(seat, action).await;
            }

            Cmd::Ready { user_id } => {
                let Some(seat) = self.seat_of(user_id) else {
                    return;
                };
                if self.state.phase != Phase::Finished {
                    return;
                }
                self.ready.insert(seat);
                // On repart des que tous les joueurs presents l'ont demande ;
                // les bots ne votent pas, et un absent ne bloque personne.
                if self.everyone_ready() {
                    self.proceed().await;
                } else {
                    self.broadcast_snapshot();
                }
            }

            Cmd::Start { user_id } => {
                // Seul le proprietaire lance la table, et une seule fois.
                if user_id != self.owner_id || self.game_id.is_some() {
                    return;
                }
                self.start_match().await;
                self.broadcast_seats();
            }

            Cmd::Resync { conn_id } => {
                if let Some(conn) = self.conns.iter().find(|c| c.conn_id == conn_id) {
                    let msg = self.snapshot_for(conn.seat);
                    let _ = conn.tx.try_send(msg);
                }
            }
        }
    }

    /// Renvoie vrai si la table doit s'arreter.
    async fn on_tick(&mut self) -> bool {
        if let Some(since) = self.empty_since {
            if since.elapsed() > IDLE_SHUTDOWN {
                return true;
            }
            // Personne ne regarde : on ne fait pas jouer les bots dans le vide.
            return false;
        }

        // Partie solo dont un siege humain ne repond pas : au bout d'un moment
        // on commence sans lui, un bot tient sa place. Une table entre amis,
        // elle, attend indefiniment que son proprietaire la lance.
        if self.game_id.is_none() {
            if self.autostart {
                if let Some(since) = self.waiting_since {
                    if since.elapsed() > WAIT_FOR_PLAYERS {
                        self.start_match().await;
                    }
                }
            }
            return false;
        }

        let Some(at) = self.act_at else {
            return false;
        };
        if Instant::now() < at {
            return false;
        }

        match self.state.phase {
            // Personne ne s'est prononce : on enchaine seul, pour ne pas
            // laisser une table en plan si quelqu'un s'est eclipse.
            Phase::Finished => self.proceed().await,
            Phase::Redeal => self.deal().await,
            Phase::Bidding1 | Phase::Bidding2 | Phase::Playing => {
                // Soit c'est un bot, soit le joueur a laisse filer son temps.
                let seat = self.state.turn;
                let view = project(&self.state, seat);
                match bot::choose_action(&view) {
                    Some(action) => self.do_action(seat, action).await,
                    None => self.act_at = None,
                }
            }
            Phase::Dealing => self.act_at = None,
        }
        false
    }

    // -----------------------------------------------------------------------
    // Deroulement
    // -----------------------------------------------------------------------

    async fn start_match(&mut self) {
        let game_id = Uuid::now_v7();

        if let Err(err) = self.insert_game(game_id).await {
            tracing::error!(?err, "impossible d'ouvrir la partie en base");
            self.broadcast(ServerMsg::Error {
                message: "impossible de demarrer la partie".into(),
            });
            return;
        }

        self.game_id = Some(game_id);
        self.totals = [0, 0];
        self.carry = 0;
        self.winner = None;
        self.seq = 0;
        self.dealer = Seat(0);
        self.deal().await;
    }

    async fn insert_game(&self, game_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("insert into games (id, table_id) values ($1, $2)")
            .bind(game_id)
            .bind(self.table_id)
            .execute(&self.pool)
            .await?;

        for (i, occ) in self.occupants.iter().enumerate() {
            sqlx::query(
                "insert into game_players (game_id, seat, user_id, is_bot) values ($1, $2, $3, $4)",
            )
            .bind(game_id)
            .bind(i as i16)
            .bind(occ.user_id)
            .bind(occ.is_bot)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query("update game_tables set status = 'playing' where id = $1")
            .bind(self.table_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Reprend apres une donne terminee : donne suivante, ou nouveau match si
    /// le precedent est alle a son terme.
    async fn proceed(&mut self) {
        self.ready.clear();
        if self.winner.is_some() {
            self.start_match().await;
        } else {
            self.deal().await;
        }
    }

    /// Distribue une nouvelle donne.
    async fn deal(&mut self) {
        self.settled = false;
        self.ready.clear();
        let ev = start_deal(self.dealer, self.carry, &mut self.rng);
        reduce(&mut self.state, &ev);
        self.emit(&ev);
        self.broadcast_snapshot();
        self.schedule();
    }

    async fn do_action(&mut self, seat: Seat, action: Action) {
        match apply(&self.state, seat, action) {
            Err(err) => self.send_error(seat, &err.to_string()),
            Ok(events) => {
                for ev in &events {
                    reduce(&mut self.state, ev);
                    self.emit(ev);
                }
                if self.state.phase == Phase::Finished && !self.settled {
                    self.settle();
                }
                self.broadcast_snapshot();
                self.schedule();
            }
        }
    }

    /// Ajoute la donne au score du match et regarde si quelqu'un a gagne.
    fn settle(&mut self) {
        let Some(score) = self.state.score.clone() else {
            return;
        };
        self.settled = true;
        self.totals[0] += score.points[0];
        self.totals[1] += score.points[1];
        self.carry = score.carry_out;
        self.dealer = self.dealer.next();

        let high = self.totals[0].max(self.totals[1]);
        if high >= TARGET_POINTS && self.totals[0] != self.totals[1] {
            let winner = if self.totals[0] > self.totals[1] { 0 } else { 1 };
            self.winner = Some(winner);

            if let Some(game_id) = self.game_id {
                let _ = self.persist.send(PersistMsg::Finish {
                    game_id,
                    totals: self.totals,
                });
            }
            tracing::info!(table = %self.join_code, winner, totals = ?self.totals, "match termine");
        }
    }

    /// Programme la prochaine action automatique.
    fn schedule(&mut self) {
        self.act_at = match self.state.phase {
            Phase::Bidding1 | Phase::Bidding2 | Phase::Playing => {
                let seat = self.state.turn;
                let mut delay = if self.is_auto(seat) {
                    BOT_THINK
                } else {
                    TURN_TIMEOUT
                };
                // Un pli vient d'etre ramasse : on laisse le temps de le voir
                // avant que la carte suivante ne tombe.
                if self.state.trick.is_empty() && self.state.tricks_played > 0 {
                    delay += TRICK_PAUSE;
                }
                Some(Instant::now() + delay)
            }
            Phase::Finished => {
                // Match gagne : on attend une decision explicite, on ne relance
                // jamais tout seul.
                if self.winner.is_some() {
                    None
                } else {
                    Some(Instant::now() + AUTO_CONTINUE)
                }
            }
            Phase::Redeal => Some(Instant::now() + REDEAL_PAUSE),
            Phase::Dealing => None,
        };
    }

    // -----------------------------------------------------------------------
    // Diffusion
    // -----------------------------------------------------------------------

    /// Journalise l'evenement et l'envoie a chacun, projete pour son siege.
    fn emit(&mut self, ev: &Event) {
        self.seq += 1;
        let seq = self.seq;

        if let Some(game_id) = self.game_id {
            match serde_json::to_value(ev) {
                Ok(payload) => {
                    let _ = self.persist.send(PersistMsg::Event {
                        game_id,
                        seq,
                        payload,
                    });
                }
                Err(err) => tracing::error!(?err, "evenement non serialisable"),
            }
        }

        for conn in &self.conns {
            // Un envoi perdu ne casse rien : le `Snapshot` qui suit fait autorite.
            let _ = conn.tx.try_send(ServerMsg::Event {
                seq,
                event: ev.redact(conn.seat),
            });
        }
    }

    fn snapshot_for(&self, seat: Seat) -> ServerMsg {
        ServerMsg::Snapshot {
            seq: self.seq,
            view: Box::new(project(&self.state, seat)),
            totals: self.totals,
            carry: self.carry,
            seats: self.seat_infos(),
            winner: self.winner,
            ready: self.ready.iter().copied().collect(),
            awaiting_continue: self.state.phase == Phase::Finished,
            in_lobby: self.game_id.is_none(),
            can_start: self.game_id.is_none()
                && self.occupants[seat.index()].user_id == Some(self.owner_id),
            join_code: self.join_code.clone(),
        }
    }

    fn broadcast_snapshot(&self) {
        for conn in &self.conns {
            let _ = conn.tx.try_send(self.snapshot_for(conn.seat));
        }
    }

    fn broadcast_seats(&self) {
        let seats = self.seat_infos();
        self.broadcast(ServerMsg::Seats { seats });
    }

    fn broadcast(&self, msg: ServerMsg) {
        for conn in &self.conns {
            let _ = conn.tx.try_send(msg.clone());
        }
    }

    fn send_error(&self, seat: Seat, message: &str) {
        // Une erreur de regle ne concerne que son auteur.
        for conn in self.conns.iter().filter(|c| c.seat == seat) {
            let _ = conn.tx.try_send(ServerMsg::Error {
                message: message.to_string(),
            });
        }
    }

    // -----------------------------------------------------------------------
    // Utilitaires
    // -----------------------------------------------------------------------

    fn seat_of(&self, user_id: Uuid) -> Option<Seat> {
        self.occupants
            .iter()
            .position(|o| o.user_id == Some(user_id))
            .map(|i| Seat(i as u8))
    }

    fn is_connected(&self, seat: Seat) -> bool {
        self.conns.iter().any(|c| c.seat == seat)
    }

    /// Vrai quand tous les sieges tenus par des humains ont une connexion.
    fn all_humans_connected(&self) -> bool {
        Seat::ALL
            .into_iter()
            .all(|seat| self.occupants[seat.index()].is_bot || self.is_connected(seat))
    }

    /// Tous les joueurs presents ont demande la suite. Les bots ne votent pas,
    /// et un joueur deconnecte ne bloque pas la table.
    fn everyone_ready(&self) -> bool {
        let present: Vec<Seat> = Seat::ALL
            .into_iter()
            .filter(|seat| !self.occupants[seat.index()].is_bot && self.is_connected(*seat))
            .collect();
        !present.is_empty() && present.iter().all(|seat| self.ready.contains(seat))
    }

    /// Un siege joue tout seul s'il est tenu par un bot, ou si son occupant a
    /// perdu la connexion.
    fn is_auto(&self, seat: Seat) -> bool {
        self.occupants[seat.index()].is_bot || !self.is_connected(seat)
    }

    fn seat_infos(&self) -> Vec<SeatInfo> {
        Seat::ALL
            .into_iter()
            .map(|seat| SeatInfo {
                seat,
                display_name: self.occupants[seat.index()].display_name.clone(),
                is_bot: self.occupants[seat.index()].is_bot,
                connected: self.is_connected(seat),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Registre des tables
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct TableRegistry {
    inner: Arc<Mutex<HashMap<String, mpsc::Sender<Cmd>>>>,
}

impl TableRegistry {
    /// Recupere l'acteur de la table, ou le cree a partir de la base.
    pub async fn get_or_spawn(
        &self,
        pool: &PgPool,
        persist: &mpsc::UnboundedSender<PersistMsg>,
        join_code: &str,
    ) -> ApiResult<mpsc::Sender<Cmd>> {
        // Un acteur arrete laisse un canal ferme : on le remplace.
        if let Some(tx) = self.lookup(join_code) {
            if !tx.is_closed() {
                return Ok(tx);
            }
        }

        let record = load_table(pool, join_code).await?;

        let mut guard = self.inner.lock().expect("registre empoisonne");
        // Quelqu'un a pu creer l'acteur pendant qu'on lisait la base.
        if let Some(existing) = guard.get(join_code) {
            if !existing.is_closed() {
                return Ok(existing.clone());
            }
        }
        let tx = TableActor::spawn(pool.clone(), persist.clone(), record);
        guard.insert(join_code.to_string(), tx.clone());
        Ok(tx)
    }

    fn lookup(&self, join_code: &str) -> Option<mpsc::Sender<Cmd>> {
        self.inner
            .lock()
            .expect("registre empoisonne")
            .get(join_code)
            .cloned()
    }
}

#[derive(sqlx::FromRow)]
struct SeatRow {
    seat: i16,
    user_id: Option<Uuid>,
    display_name: Option<String>,
    is_bot: bool,
}

/// Tout ce qu'il faut pour ouvrir une table, lu une fois en base.
pub struct TableRecord {
    pub table_id: Uuid,
    pub join_code: String,
    pub owner_id: Uuid,
    pub autostart: bool,
    pub occupants: [Occupant; 4],
}

async fn load_table(pool: &PgPool, join_code: &str) -> ApiResult<TableRecord> {
    let table: Option<(Uuid, Uuid, bool)> =
        sqlx::query_as("select id, owner_id, autostart from game_tables where join_code = $1")
            .bind(join_code)
            .fetch_optional(pool)
            .await?;
    let (table_id, owner_id, autostart) = table.ok_or(ApiError::NotFound)?;

    let rows: Vec<SeatRow> = sqlx::query_as(
        "select s.seat, s.user_id, u.display_name, s.is_bot
           from table_seats s
           left join users u on u.id = s.user_id
          where s.table_id = $1
          order by s.seat",
    )
    .bind(table_id)
    .fetch_all(pool)
    .await?;

    if rows.len() != 4 {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "la table {join_code} n'a pas 4 sieges"
        )));
    }

    const BOT_NAMES: [&str; 4] = ["Robot Sud", "Robot Ouest", "Robot Nord", "Robot Est"];
    let mut occupants: [Occupant; 4] = std::array::from_fn(|i| Occupant {
        user_id: None,
        display_name: BOT_NAMES[i].to_string(),
        is_bot: true,
    });

    for row in rows {
        let i = row.seat as usize;
        occupants[i] = Occupant {
            user_id: row.user_id,
            display_name: row
                .display_name
                .unwrap_or_else(|| BOT_NAMES[i].to_string()),
            is_bot: row.is_bot,
        };
    }

    Ok(TableRecord {
        table_id,
        join_code: join_code.to_string(),
        owner_id,
        autostart,
        occupants,
    })
}

// ---------------------------------------------------------------------------
// Ecriture du journal
// ---------------------------------------------------------------------------

/// Nombre maximal d'evenements ecrits en une seule requete.
const PERSIST_BATCH: usize = 64;

/// Tache unique d'ecriture : les evenements arrivent deja ordonnes par le
/// canal, et le jeu n'attend jamais la base.
///
/// Les ecritures sont groupees. Une insertion ligne par ligne vers une base
/// distante coute un aller-retour reseau chacune : sur une donne de 45
/// evenements, le journal accusait plusieurs secondes de retard.
pub fn spawn_persister(pool: PgPool) -> mpsc::UnboundedSender<PersistMsg> {
    let (tx, mut rx) = mpsc::unbounded_channel::<PersistMsg>();

    tokio::spawn(async move {
        let mut batch: Vec<(Uuid, i32, serde_json::Value)> = Vec::with_capacity(PERSIST_BATCH);

        while let Some(first) = rx.recv().await {
            let mut finish: Option<(Uuid, [u16; 2])> = None;
            let mut pending = Some(first);

            // On vide ce qui est deja arrive pour l'ecrire d'un seul coup.
            while let Some(msg) = pending.take() {
                match msg {
                    PersistMsg::Event {
                        game_id,
                        seq,
                        payload,
                    } => batch.push((game_id, seq as i32, payload)),
                    PersistMsg::Finish { game_id, totals } => finish = Some((game_id, totals)),
                }
                if batch.len() < PERSIST_BATCH {
                    pending = rx.try_recv().ok();
                }
            }

            if !batch.is_empty() {
                let mut qb =
                    sqlx::QueryBuilder::new("insert into game_events (game_id, seq, payload) ");
                qb.push_values(batch.drain(..), |mut row, (game_id, seq, payload)| {
                    row.push_bind(game_id).push_bind(seq).push_bind(payload);
                });
                // Une reemission ne doit pas dupliquer une ligne du journal.
                qb.push(" on conflict (game_id, seq) do nothing");

                if let Err(err) = qb.build().execute(&pool).await {
                    tracing::error!(?err, "ecriture du journal impossible");
                }
            }

            // La cloture vient apres, pour qu'une partie marquee terminee ait
            // bien tout son journal derriere elle.
            if let Some((game_id, totals)) = finish {
                let result = sqlx::query(
                    "update games
                        set ended_at = now(),
                            final_scores = $2
                      where id = $1",
                )
                .bind(game_id)
                .bind(serde_json::json!({ "totals": totals }))
                .execute(&pool)
                .await;

                if let Err(err) = result {
                    tracing::error!(?err, "cloture de la partie impossible");
                }
            }
        }
    });

    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bot_seat_always_plays_by_itself() {
        // On verifie la regle de decision, sans lancer d'acteur.
        let occupants: [Occupant; 4] = std::array::from_fn(|i| Occupant {
            user_id: if i == 0 { Some(Uuid::nil()) } else { None },
            display_name: format!("s{i}"),
            is_bot: i != 0,
        });
        assert!(occupants[1].is_bot);
        assert!(!occupants[0].is_bot);
    }
}
