//! Projection de l'etat pour un joueur donne.
//!
//! C'est la frontiere de confidentialite du jeu : un `PlayerView` ne contient
//! jamais les cartes des autres sieges, seulement leur nombre. Le serveur ne
//! doit envoyer au client que ce type (et `PublicEvent`), jamais `GameState`.

use serde::{Deserialize, Serialize};

use crate::card::{Card, Suit};
use crate::rules::legal_moves;
use crate::state::{DealScore, GameState, Phase, PlayedCard, Seat};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    pub seat: Seat,
    pub phase: Phase,
    pub turn: Seat,
    pub dealer: Seat,
    pub taker: Option<Seat>,
    pub trump: Option<Suit>,
    pub upcard: Option<Card>,
    /// Ma main, en clair.
    pub hand: Vec<Card>,
    /// Nombre de cartes de chaque siege, y compris le mien.
    pub hand_sizes: [usize; 4],
    pub trick: Vec<PlayedCard>,
    pub trick_leader: Seat,
    pub tricks_played: u8,
    pub tricks_won: [u8; 2],
    pub card_points: [u16; 2],
    /// Vrai si je detiens roi + dame d'atout.
    pub belote_mine: bool,
    /// Ce que je peux poser maintenant ; vide si ce n'est pas mon tour.
    pub legal: Vec<Card>,
    pub carry_in: u16,
    pub score: Option<DealScore>,
}

/// Construit la vue du siege `seat`.
pub fn project(state: &GameState, seat: Seat) -> PlayerView {
    PlayerView {
        seat,
        phase: state.phase,
        turn: state.turn,
        dealer: state.dealer,
        taker: state.taker,
        trump: state.trump,
        upcard: state.upcard,
        hand: state.hand(seat).to_vec(),
        hand_sizes: [
            state.hands[0].len(),
            state.hands[1].len(),
            state.hands[2].len(),
            state.hands[3].len(),
        ],
        trick: state.trick.clone(),
        trick_leader: state.trick_leader,
        tricks_played: state.tricks_played,
        tricks_won: state.tricks_won,
        card_points: state.card_points,
        belote_mine: state.belote_seat == Some(seat),
        legal: legal_moves(state, seat),
        carry_in: state.carry_in,
        score: state.score.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{reduce, start_deal};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn a_view_never_carries_another_seats_cards() {
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        let mut state = GameState::new(Seat(0), 0);
        reduce(&mut state, &start_deal(Seat(0), 0, &mut rng));

        let view = project(&state, Seat(1));
        let serialized = serde_json::to_string(&view).unwrap();

        for other in [Seat(0), Seat(2), Seat(3)] {
            for card in state.hand(other) {
                // Une carte des autres ne peut apparaitre que si je l'ai aussi
                // (impossible ici : les mains sont disjointes).
                if !state.hand(Seat(1)).contains(card) {
                    let fragment = serde_json::to_string(card).unwrap();
                    assert!(
                        !serialized.contains(&fragment),
                        "fuite de {card} dans la vue du siege 1"
                    );
                }
            }
        }
    }
}
