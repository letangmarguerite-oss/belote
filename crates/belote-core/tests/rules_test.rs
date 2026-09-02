//! Tests des regles de la belote classique.
//!
//! Chaque contrainte de jeu a son test dedie : ce sont ces regles-la qui
//! produisent l'essentiel des bugs dans une implementation de belote.

use belote_core::action::{Action, RuleError};
use belote_core::rules::{apply, legal_moves, score_deal, trick_winner};
use belote_core::state::{GameState, Phase, PlayedCard, Seat};
use belote_core::{Card, Rank, Suit};

const TRUMP: Suit = Suit::Diamonds;

fn c(suit: Suit, rank: Rank) -> Card {
    Card::new(suit, rank)
}

/// Etat en phase de jeu, entierement controle par le test.
fn playing(hands: [Vec<Card>; 4], turn: Seat, trick: Vec<(Seat, Card)>) -> GameState {
    let mut state = GameState::new(Seat(3), 0);
    state.phase = Phase::Playing;
    state.trump = Some(TRUMP);
    state.taker = Some(Seat(0));
    state.hands = hands;
    state.turn = turn;
    state.trick_leader = trick.first().map(|(s, _)| *s).unwrap_or(turn);
    state.trick = trick
        .into_iter()
        .map(|(seat, card)| PlayedCard { seat, card })
        .collect();
    state
}

fn sorted(mut cards: Vec<Card>) -> Vec<Card> {
    cards.sort();
    cards
}

// ---------------------------------------------------------------------------
// Obligations de jeu
// ---------------------------------------------------------------------------

#[test]
fn must_follow_the_led_suit() {
    let state = playing(
        [
            vec![],
            vec![
                c(Suit::Spades, Rank::Seven),
                c(Suit::Spades, Rank::Eight),
                c(Suit::Hearts, Rank::Ace),
                c(TRUMP, Rank::King),
            ],
            vec![],
            vec![],
        ],
        Seat(1),
        vec![(Seat(0), c(Suit::Spades, Rank::Ace))],
    );

    assert_eq!(
        sorted(legal_moves(&state, Seat(1))),
        sorted(vec![
            c(Suit::Spades, Rank::Seven),
            c(Suit::Spades, Rank::Eight)
        ]),
        "avoir du pique oblige a fournir, meme avec de l'atout en main"
    );
}

#[test]
fn must_trump_when_void_and_an_opponent_holds_the_trick() {
    let state = playing(
        [
            vec![],
            vec![
                c(TRUMP, Rank::Seven),
                c(Suit::Hearts, Rank::Ace),
                c(Suit::Clubs, Rank::Ten),
            ],
            vec![],
            vec![],
        ],
        Seat(1),
        vec![(Seat(0), c(Suit::Spades, Rank::Ace))],
    );

    assert_eq!(
        legal_moves(&state, Seat(1)),
        vec![c(TRUMP, Rank::Seven)],
        "sans la couleur demandee et adversaire maitre : obligation de couper"
    );
}

#[test]
fn may_discard_freely_when_the_partner_holds_the_trick() {
    let hand = vec![
        c(TRUMP, Rank::Seven),
        c(Suit::Hearts, Rank::King),
        c(Suit::Clubs, Rank::Ten),
    ];
    // Le siege 2 est le partenaire du siege 0, qui tient le pli.
    let state = playing(
        [vec![], vec![], hand.clone(), vec![]],
        Seat(2),
        vec![
            (Seat(0), c(Suit::Spades, Rank::Ace)),
            (Seat(1), c(Suit::Spades, Rank::Seven)),
        ],
    );

    assert_eq!(
        sorted(legal_moves(&state, Seat(2))),
        sorted(hand),
        "partenaire maitre : aucune obligation de couper"
    );
}

#[test]
fn must_overtrump_when_able() {
    let state = playing(
        [
            vec![],
            vec![],
            vec![
                c(TRUMP, Rank::Seven),
                c(TRUMP, Rank::Jack),
                c(Suit::Hearts, Rank::King),
            ],
            vec![],
        ],
        Seat(2),
        vec![
            (Seat(0), c(Suit::Spades, Rank::Ace)),
            (Seat(1), c(TRUMP, Rank::Eight)),
        ],
    );

    assert_eq!(
        legal_moves(&state, Seat(2)),
        vec![c(TRUMP, Rank::Jack)],
        "un adversaire a coupe : il faut monter au-dessus si on le peut"
    );
}

#[test]
fn may_undertrump_when_unable_to_overtrump() {
    let state = playing(
        [
            vec![],
            vec![],
            vec![c(TRUMP, Rank::Seven), c(Suit::Hearts, Rank::King)],
            vec![],
        ],
        Seat(2),
        vec![
            (Seat(0), c(Suit::Spades, Rank::Ace)),
            (Seat(1), c(TRUMP, Rank::Jack)),
        ],
    );

    assert_eq!(
        legal_moves(&state, Seat(2)),
        vec![c(TRUMP, Rank::Seven)],
        "on ne peut pas monter : on doit quand meme mettre de l'atout"
    );
}

#[test]
fn trump_led_forces_following_and_climbing() {
    let state = playing(
        [
            vec![],
            vec![
                c(TRUMP, Rank::Seven),
                c(TRUMP, Rank::Ace),
                c(Suit::Hearts, Rank::King),
            ],
            vec![],
            vec![],
        ],
        Seat(1),
        vec![(Seat(0), c(TRUMP, Rank::Eight))],
    );

    assert_eq!(
        legal_moves(&state, Seat(1)),
        vec![c(TRUMP, Rank::Ace)],
        "atout demande : fournir et monter, meme si le partenaire est maitre"
    );
}

#[test]
fn the_leader_is_free() {
    let hand = vec![
        c(TRUMP, Rank::Seven),
        c(Suit::Hearts, Rank::King),
        c(Suit::Spades, Rank::Ace),
    ];
    let state = playing([hand.clone(), vec![], vec![], vec![]], Seat(0), vec![]);
    assert_eq!(sorted(legal_moves(&state, Seat(0))), sorted(hand));
}

#[test]
fn a_void_hand_without_trump_may_play_anything() {
    let hand = vec![c(Suit::Hearts, Rank::King), c(Suit::Clubs, Rank::Ten)];
    let state = playing(
        [vec![], hand.clone(), vec![], vec![]],
        Seat(1),
        vec![(Seat(0), c(Suit::Spades, Rank::Ace))],
    );
    assert_eq!(sorted(legal_moves(&state, Seat(1))), sorted(hand));
}

// ---------------------------------------------------------------------------
// Validation des actions
// ---------------------------------------------------------------------------

#[test]
fn apply_rejects_an_illegal_card() {
    let state = playing(
        [
            vec![],
            vec![c(Suit::Spades, Rank::Seven), c(Suit::Hearts, Rank::Ace)],
            vec![],
            vec![],
        ],
        Seat(1),
        vec![(Seat(0), c(Suit::Spades, Rank::Ace))],
    );

    let err = apply(
        &state,
        Seat(1),
        Action::Play {
            card: c(Suit::Hearts, Rank::Ace),
        },
    )
    .unwrap_err();
    assert_eq!(err, RuleError::IllegalCard);
}

#[test]
fn apply_rejects_a_card_not_in_hand() {
    let state = playing(
        [vec![], vec![c(Suit::Spades, Rank::Seven)], vec![], vec![]],
        Seat(1),
        vec![(Seat(0), c(Suit::Spades, Rank::Ace))],
    );

    let err = apply(
        &state,
        Seat(1),
        Action::Play {
            card: c(Suit::Clubs, Rank::Ace),
        },
    )
    .unwrap_err();
    assert_eq!(err, RuleError::CardNotInHand);
}

#[test]
fn apply_rejects_a_player_acting_out_of_turn() {
    let state = playing(
        [vec![c(Suit::Spades, Rank::Ten)], vec![], vec![], vec![]],
        Seat(1),
        vec![],
    );

    let err = apply(
        &state,
        Seat(0),
        Action::Play {
            card: c(Suit::Spades, Rank::Ten),
        },
    )
    .unwrap_err();
    assert_eq!(err, RuleError::NotYourTurn);
}

// ---------------------------------------------------------------------------
// Resolution des plis
// ---------------------------------------------------------------------------

#[test]
fn the_highest_trump_takes_the_trick() {
    let trick = vec![
        PlayedCard {
            seat: Seat(0),
            card: c(Suit::Spades, Rank::Ace),
        },
        PlayedCard {
            seat: Seat(1),
            card: c(TRUMP, Rank::Seven),
        },
        PlayedCard {
            seat: Seat(2),
            card: c(Suit::Spades, Rank::Ten),
        },
        PlayedCard {
            seat: Seat(3),
            card: c(TRUMP, Rank::Eight),
        },
    ];
    assert_eq!(trick_winner(&trick, TRUMP), Some(Seat(3)));
}

#[test]
fn without_trump_the_highest_card_of_the_led_suit_takes_it() {
    let trick = vec![
        PlayedCard {
            seat: Seat(0),
            card: c(Suit::Spades, Rank::King),
        },
        PlayedCard {
            seat: Seat(1),
            card: c(Suit::Spades, Rank::Ten),
        },
        PlayedCard {
            seat: Seat(2),
            card: c(Suit::Hearts, Rank::Ace),
        },
        PlayedCard {
            seat: Seat(3),
            card: c(Suit::Spades, Rank::Nine),
        },
    ];
    // Le 10 bat le roi hors atout ; le coeur ne compte pas, il n'est pas demande.
    assert_eq!(trick_winner(&trick, TRUMP), Some(Seat(1)));
}

#[test]
fn the_trump_jack_beats_the_trump_nine() {
    let trick = vec![
        PlayedCard {
            seat: Seat(0),
            card: c(TRUMP, Rank::Nine),
        },
        PlayedCard {
            seat: Seat(1),
            card: c(TRUMP, Rank::Jack),
        },
    ];
    assert_eq!(trick_winner(&trick, TRUMP), Some(Seat(1)));
}

// ---------------------------------------------------------------------------
// Decompte
// ---------------------------------------------------------------------------

/// Etat de fin de donne fabrique de toutes pieces.
fn scored(card_points: [u16; 2], tricks: [u8; 2], belote: Option<Seat>, carry_in: u16) -> GameState {
    let mut state = GameState::new(Seat(3), carry_in);
    state.phase = Phase::Playing;
    state.trump = Some(TRUMP);
    state.taker = Some(Seat(0));
    state.card_points = card_points;
    state.tricks_won = tricks;
    state.tricks_played = tricks[0] + tricks[1];
    state.belote_seat = belote;
    state
}

#[test]
fn a_made_contract_leaves_each_team_its_points() {
    let score = score_deal(&scored([100, 62], [5, 3], None, 0));
    assert!(score.contract_made);
    assert_eq!(score.points, [100, 62]);
    assert_eq!(score.carry_out, 0);
}

#[test]
fn a_failed_contract_gives_everything_to_the_defenders() {
    let score = score_deal(&scored([70, 92], [3, 5], None, 0));
    assert!(!score.contract_made);
    assert_eq!(score.points, [0, 162], "dedans : les defenseurs prennent 162");
}

#[test]
fn belote_stays_with_its_holder_even_when_the_taker_falls() {
    // Le preneur (siege 0) a la belote mais chute : 70 + 20 = 90 < 92.
    let score = score_deal(&scored([70, 92], [3, 5], Some(Seat(0)), 0));
    assert!(!score.contract_made);
    assert_eq!(score.points, [20, 162]);
    assert_eq!(score.belote, Some(Seat(0)));
}

#[test]
fn belote_counts_towards_the_contract() {
    // Sans la belote le preneur serait dedans (76 < 86) ; avec, il passe a 96.
    let score = score_deal(&scored([76, 86], [4, 4], Some(Seat(0)), 0));
    assert!(score.contract_made);
    assert_eq!(score.points, [96, 86]);
}

#[test]
fn a_capot_is_worth_252() {
    let score = score_deal(&scored([162, 0], [8, 0], None, 0));
    assert_eq!(score.capot, Some(Seat(0).team()));
    assert_eq!(score.points, [252, 0]);
    assert!(score.contract_made);
}

#[test]
fn a_capot_by_the_defenders_is_also_worth_252() {
    let score = score_deal(&scored([0, 162], [0, 8], None, 0));
    assert_eq!(score.capot, Some(Seat(1).team()));
    assert_eq!(score.points, [0, 252]);
    assert!(!score.contract_made);
}

#[test]
fn an_exact_tie_sends_the_takers_points_to_the_pot() {
    let score = score_deal(&scored([81, 81], [4, 4], None, 0));
    assert!(score.litige);
    assert_eq!(score.points, [0, 81]);
    assert_eq!(score.carry_out, 81, "les 81 du preneur sont mis en cagnotte");
}

#[test]
fn the_pot_goes_to_the_taker_who_makes_the_contract() {
    let score = score_deal(&scored([100, 62], [5, 3], None, 81));
    assert!(score.contract_made);
    assert_eq!(score.points, [181, 62]);
    assert_eq!(score.carry_out, 0);
}

#[test]
fn the_pot_goes_to_the_defenders_when_the_taker_falls() {
    let score = score_deal(&scored([70, 92], [3, 5], None, 81));
    assert_eq!(score.points, [0, 243]);
}

#[test]
fn raw_points_always_add_up_to_162() {
    let score = score_deal(&scored([90, 72], [5, 3], None, 0));
    assert_eq!(score.raw[0] + score.raw[1], 162);
}
