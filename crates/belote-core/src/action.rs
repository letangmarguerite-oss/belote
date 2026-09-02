//! Ce qu'un joueur peut demander, et les raisons d'un refus.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::card::{Card, Suit};
use crate::state::{Phase, Seat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Premier tour : prendre la couleur de la carte retournee.
    Take,
    /// Second tour : nommer une autre couleur comme atout.
    ChooseTrump { suit: Suit },
    Pass,
    Play { card: Card },
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleError {
    #[error("ce n'est pas a ce joueur d'agir")]
    NotYourTurn,
    #[error("action impossible dans la phase {phase:?}")]
    WrongPhase { phase: Phase },
    #[error("carte absente de la main du joueur")]
    CardNotInHand,
    #[error("coup interdit par les regles de la belote")]
    IllegalCard,
    #[error("au second tour, l'atout ne peut pas etre la couleur retournee")]
    TrumpMustDiffer,
    #[error("la donne est terminee")]
    DealOver,
    #[error("siege inconnu: {seat:?}")]
    UnknownSeat { seat: Seat },
}
