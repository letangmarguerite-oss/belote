//! Tests de proprietes : on fait jouer des milliers de donnes completes par
//! des bots et on verifie les invariants qui doivent tenir quoi qu'il arrive.
//!
//! C'est ce qui attrape les regles subtiles que les tests unitaires manquent :
//! blocages, cartes dupliquees, points qui ne tombent pas juste.

use std::collections::HashSet;

use belote_core::event::Event;
use belote_core::rules::{legal_moves, reduce};
use belote_core::scoring::DEAL_TOTAL;
use belote_core::sim::{check_invariants, play_deal};
use belote_core::state::{GameState, Phase, Seat};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// En debug la simulation est lente : on en fait moins qu'en release, ou le
/// binaire `simulate` tourne sur 10 000 donnes.
const DEALS: usize = if cfg!(debug_assertions) { 1_500 } else { 10_000 };

#[test]
fn thousands_of_deals_hold_every_invariant() {
    let mut rng = ChaCha8Rng::seed_from_u64(0xBE10DE);
    let mut carry = 0u16;

    for i in 0..DEALS {
        let dealer = Seat((i % 4) as u8);
        let outcome = play_deal(dealer, carry, &mut rng)
            .unwrap_or_else(|e| panic!("donne {i} n'a pas abouti : {e}"));

        let problems = check_invariants(&outcome);
        assert!(problems.is_empty(), "donne {i} : {}", problems.join(" | "));

        carry = outcome
            .state
            .score
            .as_ref()
            .expect("donne comptee")
            .carry_out;
    }
}

#[test]
fn replaying_the_event_log_reproduces_the_state_exactly() {
    let mut rng = ChaCha8Rng::seed_from_u64(1234);

    for i in 0..200 {
        let dealer = Seat((i % 4) as u8);
        let outcome = play_deal(dealer, 0, &mut rng).expect("donne jouable");

        let mut replay = GameState::new(dealer, 0);
        for ev in &outcome.events {
            reduce(&mut replay, ev);
        }
        assert_eq!(
            replay, outcome.state,
            "donne {i} : le rejeu du journal diverge de l'etat courant"
        );
    }
}

#[test]
fn a_player_to_move_always_has_at_least_one_legal_card() {
    let mut rng = ChaCha8Rng::seed_from_u64(777);

    for i in 0..300 {
        let dealer = Seat((i % 4) as u8);
        let outcome = play_deal(dealer, 0, &mut rng).expect("donne jouable");

        // On rejoue le journal en verifiant l'invariant a chaque etape.
        let mut state = GameState::new(dealer, 0);
        for ev in &outcome.events {
            reduce(&mut state, ev);
            if state.phase == Phase::Playing && !state.hand(state.turn).is_empty() {
                assert!(
                    !legal_moves(&state, state.turn).is_empty(),
                    "donne {i} : le siege {:?} n'a aucun coup legal",
                    state.turn
                );
            }
        }
    }
}

#[test]
fn every_legal_move_is_a_card_actually_held() {
    let mut rng = ChaCha8Rng::seed_from_u64(31337);

    for i in 0..300 {
        let dealer = Seat((i % 4) as u8);
        let outcome = play_deal(dealer, 0, &mut rng).expect("donne jouable");

        let mut state = GameState::new(dealer, 0);
        for ev in &outcome.events {
            reduce(&mut state, ev);
            if state.phase == Phase::Playing {
                let hand: HashSet<_> = state.hand(state.turn).iter().copied().collect();
                for card in legal_moves(&state, state.turn) {
                    assert!(
                        hand.contains(&card),
                        "donne {i} : {card} proposee hors de la main"
                    );
                }
            }
        }
    }
}

#[test]
fn the_32_cards_are_dealt_once_and_played_once() {
    let mut rng = ChaCha8Rng::seed_from_u64(4242);

    for i in 0..300 {
        let dealer = Seat((i % 4) as u8);
        let outcome = play_deal(dealer, 0, &mut rng).expect("donne jouable");

        let played: Vec<_> = outcome
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Played { card, .. } => Some(*card),
                _ => None,
            })
            .collect();

        assert_eq!(played.len(), 32, "donne {i}");
        assert_eq!(
            played.iter().collect::<HashSet<_>>().len(),
            32,
            "donne {i} : une carte a ete posee deux fois"
        );
    }
}

#[test]
fn deal_points_always_total_162() {
    let mut rng = ChaCha8Rng::seed_from_u64(999);

    for i in 0..500 {
        let dealer = Seat((i % 4) as u8);
        let outcome = play_deal(dealer, 0, &mut rng).expect("donne jouable");
        let score = outcome.state.score.expect("donne comptee");
        assert_eq!(
            score.raw[0] + score.raw[1],
            DEAL_TOTAL,
            "donne {i} : les points des plis ne tombent pas juste"
        );
    }
}

#[test]
fn each_seat_plays_exactly_eight_cards() {
    let mut rng = ChaCha8Rng::seed_from_u64(2024);

    for i in 0..200 {
        let dealer = Seat((i % 4) as u8);
        let outcome = play_deal(dealer, 0, &mut rng).expect("donne jouable");

        let mut counts = [0usize; 4];
        for ev in &outcome.events {
            if let Event::Played { seat, .. } = ev {
                counts[seat.index()] += 1;
            }
        }
        assert_eq!(counts, [8, 8, 8, 8], "donne {i}");
    }
}

#[test]
fn a_redacted_event_never_carries_another_seats_cards() {
    let mut rng = ChaCha8Rng::seed_from_u64(5150);
    let outcome = play_deal(Seat(0), 0, &mut rng).expect("donne jouable");

    // On rejoue en suivant, pour chaque siege, ce qu'il aurait reellement vu.
    for seat in Seat::ALL {
        let mut state = GameState::new(Seat(0), 0);
        for ev in &outcome.events {
            let public = ev.redact(seat);
            let json = serde_json::to_string(&public).unwrap();

            // Les cartes citees par l'evenement public doivent etre soit les
            // miennes, soit deja publiques (posees sur la table, ou retournee).
            if let Event::Dealt { hands, .. } = ev {
                for (i, hand) in hands.iter().enumerate() {
                    if i == seat.index() {
                        continue;
                    }
                    for card in hand {
                        let fragment = serde_json::to_string(card).unwrap();
                        assert!(
                            !json.contains(&fragment),
                            "la distribution fuite {card} vers le siege {seat:?}"
                        );
                    }
                }
            }
            reduce(&mut state, ev);
        }
    }
}
