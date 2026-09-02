//! Les faits de jeu. Le log d'`Event` est la seule source de verite : l'etat
//! s'obtient en les rejouant, ce qui donne l'historique, la reconnexion et la
//! reprise apres redemarrage sans code supplementaire.

use serde::{Deserialize, Serialize};

use crate::card::{Card, Suit};
use crate::state::{DealScore, Seat};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Distribution initiale : 5 cartes par joueur (3 puis 2) et une retournee.
    Dealt {
        dealer: Seat,
        hands: [Vec<Card>; 4],
        upcard: Card,
        stock: Vec<Card>,
        carry_in: u16,
    },
    Passed {
        seat: Seat,
    },
    /// Prise, au premier tour (sur la retournee) ou au second (couleur nommee).
    Took {
        seat: Seat,
        suit: Suit,
        /// Vrai si la carte retournee revient au preneur (premier tour).
        from_upcard: bool,
    },
    /// Complement de distribution : chacun monte a 8 cartes.
    DealCompleted {
        extra: [Vec<Card>; 4],
        belote_seat: Option<Seat>,
    },
    /// Quatre passes aux deux tours : personne ne prend.
    Redeal,
    /// Roi ou dame d'atout pose par le detenteur de la belote.
    BeloteShown {
        seat: Seat,
        /// Vrai sur la seconde des deux cartes (la "rebelote").
        complete: bool,
    },
    Played {
        seat: Seat,
        card: Card,
    },
    TrickTaken {
        winner: Seat,
        points: u16,
        /// Vrai pour le 8e pli (dix de der).
        last: bool,
    },
    Scored(DealScore),
}

/// Vue d'un `Event` destinee a un joueur donne : la distribution y est reduite
/// a la main du destinataire. C'est le seul type qui doit franchir le reseau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PublicEvent {
    Dealt {
        dealer: Seat,
        /// Uniquement la main du destinataire.
        hand: Vec<Card>,
        upcard: Card,
        hand_sizes: [usize; 4],
        carry_in: u16,
    },
    Passed {
        seat: Seat,
    },
    Took {
        seat: Seat,
        suit: Suit,
        from_upcard: bool,
    },
    DealCompleted {
        /// Uniquement les cartes recues par le destinataire.
        extra: Vec<Card>,
        /// Nombre de cartes recues par chaque siege, pour tenir les compteurs.
        extra_sizes: [usize; 4],
        /// Renseigne seulement si le destinataire detient la belote.
        belote_mine: bool,
    },
    Redeal,
    BeloteShown {
        seat: Seat,
        complete: bool,
    },
    Played {
        seat: Seat,
        card: Card,
    },
    TrickTaken {
        winner: Seat,
        points: u16,
        last: bool,
    },
    Scored(DealScore),
}

impl Event {
    /// Projette un evenement pour un siege. Toute la confidentialite du jeu
    /// repose sur cette fonction : rien d'autre ne doit etre emis au client.
    pub fn redact(&self, seat: Seat) -> PublicEvent {
        match self {
            Event::Dealt {
                dealer,
                hands,
                upcard,
                carry_in,
                ..
            } => PublicEvent::Dealt {
                dealer: *dealer,
                hand: hands[seat.index()].clone(),
                upcard: *upcard,
                hand_sizes: [
                    hands[0].len(),
                    hands[1].len(),
                    hands[2].len(),
                    hands[3].len(),
                ],
                carry_in: *carry_in,
            },
            Event::Passed { seat: s } => PublicEvent::Passed { seat: *s },
            Event::Took {
                seat: s,
                suit,
                from_upcard,
            } => PublicEvent::Took {
                seat: *s,
                suit: *suit,
                from_upcard: *from_upcard,
            },
            Event::DealCompleted { extra, belote_seat } => PublicEvent::DealCompleted {
                extra: extra[seat.index()].clone(),
                extra_sizes: [extra[0].len(), extra[1].len(), extra[2].len(), extra[3].len()],
                belote_mine: *belote_seat == Some(seat),
            },
            Event::Redeal => PublicEvent::Redeal,
            Event::BeloteShown { seat: s, complete } => PublicEvent::BeloteShown {
                seat: *s,
                complete: *complete,
            },
            Event::Played { seat: s, card } => PublicEvent::Played {
                seat: *s,
                card: *card,
            },
            Event::TrickTaken {
                winner,
                points,
                last,
            } => PublicEvent::TrickTaken {
                winner: *winner,
                points: *points,
                last: *last,
            },
            Event::Scored(score) => PublicEvent::Scored(score.clone()),
        }
    }
}
