//! La logique de la belote classique.
//!
//! Deux fonctions seulement modifient le cours du jeu :
//!   - `apply`  : valide une action et produit les `Event` correspondants ;
//!   - `reduce` : applique un `Event` a l'etat (et sert aussi au rejeu).
//!
//! `legal_moves` est l'unique source de verite sur ce qui est jouable. `apply`
//! refuse systematiquement toute carte absente de `legal_moves`, ce qui rend
//! impossible une divergence entre l'affichage et la validation.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::action::{Action, RuleError};
use crate::card::{Card, Rank, Suit};
use crate::event::Event;
use crate::scoring::{
    card_points, card_strength, BELOTE_BONUS, CAPOT_POINTS, DEAL_TOTAL, LAST_TRICK_BONUS,
};
use crate::state::{DealScore, GameState, Phase, PlayedCard, Seat};

// ---------------------------------------------------------------------------
// Distribution
// ---------------------------------------------------------------------------

/// Melange et distribue : 3 puis 2 cartes a chacun, une carte retournee,
/// 11 cartes en reserve pour le complement d'apres encheres.
pub fn start_deal<R: Rng>(dealer: Seat, carry_in: u16, rng: &mut R) -> Event {
    let mut deck = Card::deck();
    deck.shuffle(rng);

    let mut hands: [Vec<Card>; 4] = Default::default();
    let mut cursor = 0usize;
    for batch in [3usize, 2] {
        let mut seat = dealer.next();
        for _ in 0..4 {
            hands[seat.index()].extend_from_slice(&deck[cursor..cursor + batch]);
            cursor += batch;
            seat = seat.next();
        }
    }
    let upcard = deck[cursor];
    cursor += 1;
    let stock = deck[cursor..].to_vec();
    debug_assert_eq!(stock.len(), 11);

    Event::Dealt {
        dealer,
        hands,
        upcard,
        stock,
        carry_in,
    }
}

// ---------------------------------------------------------------------------
// Coups legaux
// ---------------------------------------------------------------------------

/// Le gagnant provisoire du pli en cours. `None` si le pli est vide.
pub fn current_winner(state: &GameState) -> Option<Seat> {
    let trump = state.trump?;
    trick_winner(&state.trick, trump)
}

/// Le gagnant d'un pli : le plus fort atout s'il y en a, sinon la plus forte
/// carte de la couleur demandee.
pub fn trick_winner(trick: &[PlayedCard], trump: Suit) -> Option<Seat> {
    let led = trick.first()?.card.suit;
    let has_trump = trick.iter().any(|p| p.card.suit == trump);
    let relevant = if has_trump { trump } else { led };

    trick
        .iter()
        .filter(|p| p.card.suit == relevant)
        .max_by_key(|p| card_strength(p.card.rank, relevant == trump))
        .map(|p| p.seat)
}

/// Somme des points d'un pli (hors dix de der).
pub fn trick_points(trick: &[PlayedCard], trump: Suit) -> u16 {
    trick
        .iter()
        .map(|p| card_points(p.card.rank, p.card.suit == trump))
        .sum()
}

/// Les cartes que `seat` peut legalement poser maintenant.
///
/// Trois contraintes, dans cet ordre :
///   1. fournir la couleur demandee si on la possede ;
///   2. sinon couper, sauf si le partenaire est deja maitre du pli ;
///   3. quand on met de l'atout, monter au-dessus du plus fort atout du pli
///      si on le peut.
pub fn legal_moves(state: &GameState, seat: Seat) -> Vec<Card> {
    if state.phase != Phase::Playing || state.turn != seat {
        return Vec::new();
    }
    let hand = state.hand(seat);
    let Some(trump) = state.trump else {
        return Vec::new();
    };

    // Entameur : totalement libre.
    let Some(led) = state.led_suit() else {
        return hand.to_vec();
    };

    let trumps: Vec<Card> = hand.iter().copied().filter(|c| c.suit == trump).collect();
    let best_trump_played: Option<u8> = state
        .trick
        .iter()
        .filter(|p| p.card.suit == trump)
        .map(|p| card_strength(p.card.rank, true))
        .max();

    // Monter a l'atout si possible, sinon n'importe quel atout ("sous-couper").
    let climb = |trumps: Vec<Card>| -> Vec<Card> {
        let higher: Vec<Card> = trumps
            .iter()
            .copied()
            .filter(|c| Some(card_strength(c.rank, true)) > best_trump_played)
            .collect();
        if higher.is_empty() {
            trumps
        } else {
            higher
        }
    };

    if led == trump {
        // Atout demande : fournir de l'atout, et monter si on peut.
        return if trumps.is_empty() {
            hand.to_vec()
        } else {
            climb(trumps)
        };
    }

    let in_suit: Vec<Card> = hand.iter().copied().filter(|c| c.suit == led).collect();
    if !in_suit.is_empty() {
        // Obligation de fournir, sans obligation de monter hors atout.
        return in_suit;
    }

    // Le partenaire tient le pli : on est libre de se defausser.
    if current_winner(state).map(|w| w.partner()) == Some(seat) {
        return hand.to_vec();
    }

    if trumps.is_empty() {
        return hand.to_vec();
    }
    climb(trumps)
}

// ---------------------------------------------------------------------------
// Application des actions
// ---------------------------------------------------------------------------

/// Valide une action et renvoie les evenements qu'elle produit.
/// N'altere jamais `state` : l'appelant les passe ensuite a `reduce`.
pub fn apply(state: &GameState, seat: Seat, action: Action) -> Result<Vec<Event>, RuleError> {
    if state.is_over() {
        return Err(RuleError::DealOver);
    }
    if state.turn != seat {
        return Err(RuleError::NotYourTurn);
    }

    match action {
        Action::Pass => {
            if !matches!(state.phase, Phase::Bidding1 | Phase::Bidding2) {
                return Err(RuleError::WrongPhase { phase: state.phase });
            }
            let mut work = state.clone();
            let mut events = vec![Event::Passed { seat }];
            reduce(&mut work, &events[0]);
            if work.phase == Phase::Bidding2 && work.passes == 4 {
                let ev = Event::Redeal;
                reduce(&mut work, &ev);
                events.push(ev);
            }
            Ok(events)
        }

        Action::Take => {
            if state.phase != Phase::Bidding1 {
                return Err(RuleError::WrongPhase { phase: state.phase });
            }
            let suit = state.upcard.expect("carte retournee presente en Bidding1").suit;
            Ok(take_events(state, seat, suit, true))
        }

        Action::ChooseTrump { suit } => {
            if state.phase != Phase::Bidding2 {
                return Err(RuleError::WrongPhase { phase: state.phase });
            }
            let upcard = state.upcard.expect("carte retournee presente en Bidding2");
            if suit == upcard.suit {
                return Err(RuleError::TrumpMustDiffer);
            }
            Ok(take_events(state, seat, suit, false))
        }

        Action::Play { card } => {
            if state.phase != Phase::Playing {
                return Err(RuleError::WrongPhase { phase: state.phase });
            }
            if !state.hand(seat).contains(&card) {
                return Err(RuleError::CardNotInHand);
            }
            if !legal_moves(state, seat).contains(&card) {
                return Err(RuleError::IllegalCard);
            }
            Ok(play_events(state, seat, card))
        }
    }
}

/// Prise : l'atout est fixe, le preneur ramasse la retournee, puis chacun est
/// complete a 8 cartes.
fn take_events(state: &GameState, seat: Seat, suit: Suit, from_upcard: bool) -> Vec<Event> {
    let upcard = state.upcard.expect("carte retournee");
    let mut extra: [Vec<Card>; 4] = Default::default();
    let mut cursor = 0usize;

    let mut s = state.dealer.next();
    for _ in 0..4 {
        // Le preneur a deja la retournee : il ne prend que 2 cartes en reserve.
        let count = if s == seat { 2 } else { 3 };
        extra[s.index()].extend_from_slice(&state.stock[cursor..cursor + count]);
        cursor += count;
        s = s.next();
    }
    extra[seat.index()].push(upcard);
    debug_assert_eq!(cursor, state.stock.len());

    // Detenteur de la belote : roi ET dame d'atout dans la meme main finale.
    let belote_seat = Seat::ALL.into_iter().find(|s| {
        let full: Vec<Card> = state
            .hand(*s)
            .iter()
            .chain(extra[s.index()].iter())
            .copied()
            .collect();
        full.contains(&Card::new(suit, Rank::King)) && full.contains(&Card::new(suit, Rank::Queen))
    });

    vec![
        Event::Took {
            seat,
            suit,
            from_upcard,
        },
        Event::DealCompleted { extra, belote_seat },
    ]
}

fn play_events(state: &GameState, seat: Seat, card: Card) -> Vec<Event> {
    let trump = state.trump.expect("atout fixe en phase de jeu");
    let mut work = state.clone();
    let mut events = Vec::new();

    // Belote / rebelote : annoncee en posant le roi ou la dame d'atout.
    if work.belote_seat == Some(seat)
        && card.suit == trump
        && matches!(card.rank, Rank::King | Rank::Queen)
    {
        let complete = work.belote_shown == 1;
        emit(&mut work, &mut events, Event::BeloteShown { seat, complete });
    }

    emit(&mut work, &mut events, Event::Played { seat, card });

    if work.trick.len() == 4 {
        let last = work.tricks_played == 7;
        let winner = trick_winner(&work.trick, trump).expect("pli non vide");
        let points = trick_points(&work.trick, trump) + if last { LAST_TRICK_BONUS } else { 0 };
        emit(
            &mut work,
            &mut events,
            Event::TrickTaken {
                winner,
                points,
                last,
            },
        );
        if last {
            let score = score_deal(&work);
            emit(&mut work, &mut events, Event::Scored(score));
        }
    }

    events
}

/// Applique l'evenement a l'etat de travail et l'ajoute au journal, pour que
/// l'evenement suivant soit calcule sur un etat deja a jour.
fn emit(work: &mut GameState, events: &mut Vec<Event>, ev: Event) {
    reduce(work, &ev);
    events.push(ev);
}

// ---------------------------------------------------------------------------
// Reduction
// ---------------------------------------------------------------------------

/// Applique un evenement a l'etat. Doit rester total et deterministe : c'est
/// aussi la fonction utilisee pour rejouer une partie depuis le journal.
pub fn reduce(state: &mut GameState, event: &Event) {
    match event {
        Event::Dealt {
            dealer,
            hands,
            upcard,
            stock,
            carry_in,
        } => {
            *state = GameState::new(*dealer, *carry_in);
            state.hands = hands.clone();
            state.upcard = Some(*upcard);
            state.stock = stock.clone();
            state.phase = Phase::Bidding1;
            state.turn = dealer.next();
        }

        Event::Passed { .. } => {
            state.passes += 1;
            state.turn = state.turn.next();
            if state.passes == 4 && state.phase == Phase::Bidding1 {
                state.phase = Phase::Bidding2;
                state.passes = 0;
                state.turn = state.dealer.next();
            }
        }

        Event::Redeal => {
            state.phase = Phase::Redeal;
        }

        Event::Took { seat, suit, .. } => {
            state.trump = Some(*suit);
            state.taker = Some(*seat);
        }

        Event::DealCompleted { extra, belote_seat } => {
            for s in Seat::ALL {
                state.hands[s.index()].extend_from_slice(&extra[s.index()]);
            }
            state.stock.clear();
            state.upcard = None;
            state.belote_seat = *belote_seat;
            state.phase = Phase::Playing;
            state.turn = state.dealer.next();
            state.trick_leader = state.dealer.next();
        }

        Event::BeloteShown { .. } => {
            state.belote_shown += 1;
        }

        Event::Played { seat, card } => {
            let hand = &mut state.hands[seat.index()];
            if let Some(pos) = hand.iter().position(|c| c == card) {
                hand.remove(pos);
            }
            state.trick.push(PlayedCard {
                seat: *seat,
                card: *card,
            });
            state.turn = seat.next();
        }

        Event::TrickTaken { winner, points, .. } => {
            let team = winner.team().index();
            state.card_points[team] += points;
            state.tricks_won[team] += 1;
            state.tricks_played += 1;
            state.trick.clear();
            state.trick_leader = *winner;
            state.turn = *winner;
        }

        Event::Scored(score) => {
            state.score = Some(score.clone());
            state.phase = Phase::Finished;
        }
    }
}

// ---------------------------------------------------------------------------
// Decompte
// ---------------------------------------------------------------------------

/// Compte la donne une fois les 8 plis joues.
pub fn score_deal(state: &GameState) -> DealScore {
    let taker = state.taker.expect("un preneur existe si la donne a ete jouee");
    let trump = state.trump.expect("un atout existe si la donne a ete jouee");
    let tt = taker.team().index();
    let dt = 1 - tt;

    let raw = state.card_points;

    let capot = if state.tricks_won[tt] == 8 {
        Some(taker.team())
    } else if state.tricks_won[dt] == 8 {
        Some(taker.team().other())
    } else {
        None
    };

    let belote_of = |team: usize| -> u16 {
        match state.belote_seat {
            Some(s) if s.team().index() == team => BELOTE_BONUS,
            _ => 0,
        }
    };

    // Totaux servant a juger le contrat : plis (ou capot) + belote.
    let mut total = match capot {
        Some(team) => {
            let mut t = [0u16; 2];
            t[team.index()] = CAPOT_POINTS;
            t
        }
        None => raw,
    };
    total[0] += belote_of(0);
    total[1] += belote_of(1);

    let mut points = [0u16; 2];
    let mut contract_made = false;
    let mut litige = false;
    let mut carry_out = 0u16;

    if total[tt] > total[dt] {
        // Contrat rempli : chacun garde ses points, la cagnotte va au preneur.
        contract_made = true;
        points = total;
        points[tt] += state.carry_in;
    } else if total[tt] < total[dt] {
        // Dedans : les defenseurs ramassent tout, la belote reste a son detenteur.
        let base = if capot == Some(taker.team().other()) {
            CAPOT_POINTS
        } else {
            DEAL_TOTAL
        };
        points[tt] = belote_of(tt);
        points[dt] = base + belote_of(dt) + state.carry_in;
    } else {
        // Litige : les points du preneur partent en cagnotte.
        litige = true;
        points[tt] = 0;
        points[dt] = total[dt];
        carry_out = total[tt] + state.carry_in;
    }

    DealScore {
        points,
        raw,
        taker,
        trump,
        belote: state.belote_seat,
        capot,
        contract_made,
        litige,
        carry_out,
    }
}
