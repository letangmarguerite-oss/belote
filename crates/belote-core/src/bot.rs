//! Joueur automatique.
//!
//! Le bot ne recoit qu'un `PlayerView` : il voit exactement ce que voit un
//! humain. Il ne peut donc pas tricher, et sa logique se teste comme celle
//! d'un client ordinaire.

use crate::action::Action;
use crate::card::{Card, Rank, Suit};
use crate::scoring::{card_points, card_strength};
use crate::state::{Phase, PlayedCard, Seat};
use crate::view::PlayerView;

/// Seuil de prise au premier tour (couleur imposee par la retournee).
const TAKE_THRESHOLD_ROUND1: u16 = 48;
/// Seuil au second tour, ou l'on choisit librement sa couleur.
const TAKE_THRESHOLD_ROUND2: u16 = 52;

/// Decide de l'action a jouer. `None` si ce n'est pas au bot d'agir.
pub fn choose_action(view: &PlayerView) -> Option<Action> {
    if view.turn != view.seat {
        return None;
    }
    match view.phase {
        Phase::Bidding1 => Some(bid_round1(view)),
        Phase::Bidding2 => Some(bid_round2(view)),
        Phase::Playing => choose_card(view).map(|card| Action::Play { card }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Encheres
// ---------------------------------------------------------------------------

/// Valeur estimee d'une main pour un atout donne : points bruts, prime de
/// controle par atout detenu, prime pour les as exterieurs.
fn evaluate(hand: &[Card], trump: Suit) -> u16 {
    let mut score = 0;
    for card in hand {
        let is_trump = card.suit == trump;
        score += card_points(card.rank, is_trump);
        if is_trump {
            score += 5;
        } else if card.rank == Rank::Ace {
            score += 3;
        }
    }
    // Le valet et le neuf d'atout font gagner des plis, pas seulement des points.
    if hand.contains(&Card::new(trump, Rank::Jack)) {
        score += 8;
    }
    if hand.contains(&Card::new(trump, Rank::Nine)) {
        score += 4;
    }
    score
}

fn bid_round1(view: &PlayerView) -> Action {
    let Some(upcard) = view.upcard else {
        return Action::Pass;
    };
    // Le preneur ramasse la retournee : on l'inclut dans l'evaluation.
    let mut hand = view.hand.clone();
    hand.push(upcard);
    if evaluate(&hand, upcard.suit) >= TAKE_THRESHOLD_ROUND1 {
        Action::Take
    } else {
        Action::Pass
    }
}

fn bid_round2(view: &PlayerView) -> Action {
    let Some(upcard) = view.upcard else {
        return Action::Pass;
    };
    let mut hand = view.hand.clone();
    hand.push(upcard);

    let best = Suit::ALL
        .into_iter()
        .filter(|s| *s != upcard.suit)
        .map(|s| (evaluate(&hand, s), s))
        .max_by_key(|(score, _)| *score);

    match best {
        Some((score, suit)) if score >= TAKE_THRESHOLD_ROUND2 => Action::ChooseTrump { suit },
        _ => Action::Pass,
    }
}

// ---------------------------------------------------------------------------
// Jeu de la carte
// ---------------------------------------------------------------------------

fn choose_card(view: &PlayerView) -> Option<Card> {
    if view.legal.is_empty() {
        return None;
    }
    let trump = view.trump?;

    if view.trick.is_empty() {
        return Some(lead(view, trump));
    }

    let winner = current_winner(&view.trick, trump);
    let partner_leads = winner == Some(view.seat.partner());
    let pot: u16 = view
        .trick
        .iter()
        .map(|p| card_points(p.card.rank, p.card.suit == trump))
        .sum();

    if partner_leads {
        // Le partenaire tient : on charge le pli avec ce qui rapporte, sans
        // gaspiller d'atout.
        let charge = view
            .legal
            .iter()
            .copied()
            .filter(|c| c.suit != trump)
            .max_by_key(|c| card_points(c.rank, false));
        return Some(charge.unwrap_or_else(|| cheapest(&view.legal, trump)));
    }

    // Sinon : prendre la main au moindre cout si c'est possible.
    let winning: Vec<Card> = view
        .legal
        .iter()
        .copied()
        .filter(|c| beats(view, *c, trump))
        .collect();

    if !winning.is_empty() {
        // Si le pli est maigre et qu'il faudrait couper cher, on economise.
        let cheap_win = winning
            .iter()
            .copied()
            .min_by_key(|c| (card_points(c.rank, c.suit == trump), card_strength(c.rank, c.suit == trump)))
            .expect("liste non vide");
        if pot >= 8 || cheap_win.suit != trump || view.tricks_played >= 5 {
            return Some(cheap_win);
        }
    }

    Some(cheapest(&view.legal, trump))
}

fn lead(view: &PlayerView, trump: Suit) -> Card {
    // Preneur en debut de donne : on fait tomber l'atout adverse.
    if view.taker == Some(view.seat) && view.tricks_played < 2 {
        if let Some(card) = view
            .legal
            .iter()
            .copied()
            .filter(|c| c.suit == trump)
            .max_by_key(|c| card_strength(c.rank, true))
        {
            return card;
        }
    }
    // Sinon un as exterieur, sinon la carte la moins chere.
    view.legal
        .iter()
        .copied()
        .filter(|c| c.suit != trump && c.rank == Rank::Ace)
        .next()
        .unwrap_or_else(|| cheapest(&view.legal, trump))
}

/// La carte la moins couteuse a lacher : d'abord les points, puis la force.
fn cheapest(cards: &[Card], trump: Suit) -> Card {
    cards
        .iter()
        .copied()
        .min_by_key(|c| {
            let is_trump = c.suit == trump;
            (
                is_trump as u8,
                card_points(c.rank, is_trump),
                card_strength(c.rank, is_trump),
            )
        })
        .expect("au moins une carte legale")
}

/// Cette carte remporterait-elle le pli si on la posait maintenant ?
fn beats(view: &PlayerView, card: Card, trump: Suit) -> bool {
    let mut trick = view.trick.clone();
    trick.push(PlayedCard {
        seat: view.seat,
        card,
    });
    current_winner(&trick, trump) == Some(view.seat)
}

fn current_winner(trick: &[PlayedCard], trump: Suit) -> Option<Seat> {
    crate::rules::trick_winner(trick, trump)
}
