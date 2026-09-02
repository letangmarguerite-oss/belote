//! Les 32 cartes du jeu de belote.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

    pub fn symbol(self) -> char {
        match self {
            Suit::Clubs => '\u{2663}',
            Suit::Diamonds => '\u{2666}',
            Suit::Hearts => '\u{2665}',
            Suit::Spades => '\u{2660}',
        }
    }

    pub fn name_fr(self) -> &'static str {
        match self {
            Suit::Clubs => "trefle",
            Suit::Diamonds => "carreau",
            Suit::Hearts => "coeur",
            Suit::Spades => "pique",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rank {
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub const ALL: [Rank; 8] = [
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    /// Etiquette francaise courte : 7 8 9 10 V D R A
    pub fn label_fr(self) -> &'static str {
        match self {
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "V",
            Rank::Queen => "D",
            Rank::King => "R",
            Rank::Ace => "A",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub const fn new(suit: Suit, rank: Rank) -> Self {
        Card { suit, rank }
    }

    /// Le paquet complet, dans un ordre canonique et stable.
    pub fn deck() -> Vec<Card> {
        let mut cards = Vec::with_capacity(32);
        for suit in Suit::ALL {
            for rank in Rank::ALL {
                cards.push(Card::new(suit, rank));
            }
        }
        cards
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank.label_fr(), self.suit.symbol())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn deck_has_32_distinct_cards() {
        let deck = Card::deck();
        assert_eq!(deck.len(), 32);
        assert_eq!(deck.iter().copied().collect::<HashSet<_>>().len(), 32);
    }
}
