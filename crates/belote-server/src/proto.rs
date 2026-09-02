//! Protocole WebSocket.
//!
//! Choix structurant : apres chaque changement, le serveur renvoie un
//! `Snapshot` complet de la vue du joueur, en plus de l'evenement. Le client
//! n'a donc aucune regle de belote a reimplementer en TypeScript : il affiche
//! l'instantane, et n'utilise les evenements que pour animer. Une `PlayerView`
//! pese moins de 2 Ko, le cout reseau est negligeable.

use belote_core::{Action, PlayerView, PublicEvent, Seat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    /// Une action de jeu : prendre, passer, choisir l'atout, poser une carte.
    Act { action: Action },
    /// Redemande l'etat complet, apres une coupure reseau par exemple.
    Resync,
    Ping,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    /// Premier message : qui je suis a cette table.
    Welcome {
        seat: Seat,
        join_code: String,
        target: u16,
    },
    /// Etat de reference. Fait autorite sur tout le reste.
    Snapshot {
        seq: u32,
        view: Box<PlayerView>,
        /// Score du match, cumule sur les donnes precedentes.
        totals: [u16; 2],
        /// Cagnotte en attente, issue d'un litige.
        carry: u16,
        /// Qui occupe les sieges, et qui est connecte.
        seats: Vec<SeatInfo>,
        /// Renseigne quand le match est fini.
        winner: Option<u8>,
    },
    /// Ce qui vient de se passer, deja projete pour ce destinataire.
    Event { seq: u32, event: PublicEvent },
    /// Un joueur arrive ou part.
    Seats { seats: Vec<SeatInfo> },
    /// Action refusee. N'est envoye qu'a son auteur.
    Error { message: String },
    Pong,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeatInfo {
    pub seat: Seat,
    pub display_name: String,
    pub is_bot: bool,
    /// Faux si le joueur humain a perdu la connexion : un bot le remplace.
    pub connected: bool,
}
