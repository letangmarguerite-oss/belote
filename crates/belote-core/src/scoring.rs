//! Bareme de la belote classique.
//!
//! Atout      : V 20 | 9 14 | A 11 | 10 10 | R 4 | D 3 | 8 0 | 7 0  => 62
//! Hors atout : A 11 | 10 10 | R 4 | D 3 | V 2 | 9 0 | 8 0 | 7 0    => 30
//!
//! Total des plis = 62 + 3 x 30 = 152, + 10 de der = 162.

use crate::card::Rank;

/// Points d'une carte selon qu'elle est a l'atout ou non.
pub fn card_points(rank: Rank, trump: bool) -> u16 {
    if trump {
        match rank {
            Rank::Jack => 20,
            Rank::Nine => 14,
            Rank::Ace => 11,
            Rank::Ten => 10,
            Rank::King => 4,
            Rank::Queen => 3,
            Rank::Eight | Rank::Seven => 0,
        }
    } else {
        match rank {
            Rank::Ace => 11,
            Rank::Ten => 10,
            Rank::King => 4,
            Rank::Queen => 3,
            Rank::Jack => 2,
            Rank::Nine | Rank::Eight | Rank::Seven => 0,
        }
    }
}

/// Force relative au sein d'une meme couleur. Croissante : 0 = plus faible.
///
/// Atout      : 7 < 8 < D < R < 10 < A < 9 < V
/// Hors atout : 7 < 8 < 9 < V < D < R < 10 < A
pub fn card_strength(rank: Rank, trump: bool) -> u8 {
    if trump {
        match rank {
            Rank::Seven => 0,
            Rank::Eight => 1,
            Rank::Queen => 2,
            Rank::King => 3,
            Rank::Ten => 4,
            Rank::Ace => 5,
            Rank::Nine => 6,
            Rank::Jack => 7,
        }
    } else {
        match rank {
            Rank::Seven => 0,
            Rank::Eight => 1,
            Rank::Nine => 2,
            Rank::Jack => 3,
            Rank::Queen => 4,
            Rank::King => 5,
            Rank::Ten => 6,
            Rank::Ace => 7,
        }
    }
}

/// Points des plis pour une donne complete, hors belote : 152.
pub const TRICK_POINTS_TOTAL: u16 = 152;
/// Prime du dernier pli.
pub const LAST_TRICK_BONUS: u16 = 10;
/// Total distribuable sur une donne ordinaire.
pub const DEAL_TOTAL: u16 = TRICK_POINTS_TOTAL + LAST_TRICK_BONUS; // 162
/// Prime belote/rebelote (roi + dame d'atout).
pub const BELOTE_BONUS: u16 = 20;
/// Valeur forfaitaire d'un capot (les 8 plis).
pub const CAPOT_POINTS: u16 = 252;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Rank;

    #[test]
    fn suit_totals_match_the_official_barem() {
        let trump: u16 = Rank::ALL.iter().map(|r| card_points(*r, true)).sum();
        let plain: u16 = Rank::ALL.iter().map(|r| card_points(*r, false)).sum();
        assert_eq!(trump, 62);
        assert_eq!(plain, 30);
        assert_eq!(trump + 3 * plain, TRICK_POINTS_TOTAL);
    }

    #[test]
    fn strengths_are_a_total_order_within_a_suit() {
        for trump in [true, false] {
            let mut seen: Vec<u8> = Rank::ALL.iter().map(|r| card_strength(*r, trump)).collect();
            seen.sort_unstable();
            assert_eq!(seen, (0..8).collect::<Vec<u8>>());
        }
    }
}
