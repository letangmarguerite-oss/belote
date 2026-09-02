//! Etat d'une donne. Structure pure : aucune I/O, aucun async.
//!
//! L'etat n'est jamais mute directement par l'exterieur : il est reconstruit en
//! rejouant des `Event` via `rules::reduce`.

use serde::{Deserialize, Serialize};

use crate::card::{Card, Suit};

/// Siege a la table. 0 = Sud, 1 = Ouest, 2 = Nord, 3 = Est.
/// Equipes : {0, 2} contre {1, 3}.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Seat(pub u8);

impl Seat {
    pub const ALL: [Seat; 4] = [Seat(0), Seat(1), Seat(2), Seat(3)];

    pub fn index(self) -> usize {
        self.0 as usize
    }

    /// Le joueur suivant dans le sens du jeu.
    pub fn next(self) -> Seat {
        Seat((self.0 + 1) % 4)
    }

    pub fn partner(self) -> Seat {
        Seat((self.0 + 2) % 4)
    }

    pub fn team(self) -> Team {
        Team(self.0 % 2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Team(pub u8);

impl Team {
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub fn other(self) -> Team {
        Team(1 - self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Avant la distribution.
    Dealing,
    /// Premier tour d'encheres : prendre la carte retournee, ou passer.
    Bidding1,
    /// Second tour : choisir une autre couleur, ou passer.
    Bidding2,
    /// Les 8 plis.
    Playing,
    /// Donne terminee et comptee.
    Finished,
    /// Quatre passes aux deux tours : il faut redistribuer.
    Redeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayedCard {
    pub seat: Seat,
    pub card: Card,
}

/// Detail du decompte d'une donne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DealScore {
    /// Points finalement attribues a chaque equipe.
    pub points: [u16; 2],
    /// Points bruts des plis, dix de der inclus, avant contrat et belote.
    pub raw: [u16; 2],
    pub taker: Seat,
    pub trump: Suit,
    /// Siege detenant roi + dame d'atout, le cas echeant.
    pub belote: Option<Seat>,
    pub capot: Option<Team>,
    /// Le preneur a-t-il rempli son contrat ?
    pub contract_made: bool,
    /// Egalite parfaite (81/81) : les points du preneur partent en cagnotte.
    pub litige: bool,
    /// Cagnotte reportee sur la donne suivante.
    pub carry_out: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub phase: Phase,
    pub dealer: Seat,
    /// A qui d'agir.
    pub turn: Seat,
    pub hands: [Vec<Card>; 4],
    /// Carte retournee pendant les encheres.
    pub upcard: Option<Card>,
    /// Cartes non encore distribuees.
    pub stock: Vec<Card>,
    pub trump: Option<Suit>,
    pub taker: Option<Seat>,
    /// Nombre de passes consecutives dans le tour d'encheres courant.
    pub passes: u8,
    pub trick: Vec<PlayedCard>,
    pub trick_leader: Seat,
    pub tricks_played: u8,
    pub tricks_won: [u8; 2],
    /// Points des plis ramasses, dix de der inclus une fois le 8e pli joue.
    pub card_points: [u16; 2],
    /// Siege detenant roi + dame d'atout apres distribution complete.
    pub belote_seat: Option<Seat>,
    /// Combien des deux cartes de belote ont deja ete jouees (0, 1 ou 2).
    pub belote_shown: u8,
    /// Cagnotte heritee d'un litige precedent.
    pub carry_in: u16,
    pub score: Option<DealScore>,
}

impl GameState {
    /// Etat vierge, avant toute distribution.
    pub fn new(dealer: Seat, carry_in: u16) -> Self {
        GameState {
            phase: Phase::Dealing,
            dealer,
            turn: dealer.next(),
            hands: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            upcard: None,
            stock: Vec::new(),
            trump: None,
            taker: None,
            passes: 0,
            trick: Vec::new(),
            trick_leader: dealer.next(),
            tricks_played: 0,
            tricks_won: [0, 0],
            card_points: [0, 0],
            belote_seat: None,
            belote_shown: 0,
            carry_in,
            score: None,
        }
    }

    pub fn hand(&self, seat: Seat) -> &[Card] {
        &self.hands[seat.index()]
    }

    /// Couleur demandee au pli en cours.
    pub fn led_suit(&self) -> Option<Suit> {
        self.trick.first().map(|p| p.card.suit)
    }

    pub fn is_over(&self) -> bool {
        matches!(self.phase, Phase::Finished | Phase::Redeal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seats_rotate_and_pair_correctly() {
        assert_eq!(Seat(3).next(), Seat(0));
        assert_eq!(Seat(0).partner(), Seat(2));
        assert_eq!(Seat(1).partner(), Seat(3));
        assert_eq!(Seat(0).team(), Seat(2).team());
        assert_ne!(Seat(0).team(), Seat(1).team());
    }
}
