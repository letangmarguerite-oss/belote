// Presentation des cartes : etiquettes francaises, couleurs, tri.

import type { Card, Rank, Seat, Suit } from "./types";

export const SUIT_SYMBOL: Record<Suit, string> = {
  clubs: "♣",
  diamonds: "♦",
  hearts: "♥",
  spades: "♠",
};

export const SUIT_LABEL: Record<Suit, string> = {
  clubs: "Trefle",
  diamonds: "Carreau",
  hearts: "Coeur",
  spades: "Pique",
};

export const RANK_LABEL: Record<Rank, string> = {
  seven: "7",
  eight: "8",
  nine: "9",
  ten: "10",
  jack: "V",
  queen: "D",
  king: "R",
  ace: "A",
};

export const isRed = (suit: Suit) => suit === "hearts" || suit === "diamonds";

export const cardKey = (card: Card) => `${card.rank}-${card.suit}`;

export const sameCard = (a: Card, b: Card) =>
  a.suit === b.suit && a.rank === b.rank;

export const hasCard = (cards: Card[], card: Card) =>
  cards.some((c) => sameCard(c, card));

/** Ordre d'affichage en main : par couleur, puis par force au sein de la couleur. */
const SUIT_ORDER: Suit[] = ["spades", "hearts", "clubs", "diamonds"];

const TRUMP_STRENGTH: Record<Rank, number> = {
  seven: 0,
  eight: 1,
  queen: 2,
  king: 3,
  ten: 4,
  ace: 5,
  nine: 6,
  jack: 7,
};

const PLAIN_STRENGTH: Record<Rank, number> = {
  seven: 0,
  eight: 1,
  nine: 2,
  jack: 3,
  queen: 4,
  king: 5,
  ten: 6,
  ace: 7,
};

/** Trie la main pour l'oeil : couleurs groupees, atout en premier. */
export function sortHand(hand: Card[], trump: Suit | null): Card[] {
  const suitRank = (suit: Suit) =>
    suit === trump ? -1 : SUIT_ORDER.indexOf(suit);

  return [...hand].sort((a, b) => {
    const bySuit = suitRank(a.suit) - suitRank(b.suit);
    if (bySuit !== 0) return bySuit;
    const table = a.suit === trump ? TRUMP_STRENGTH : PLAIN_STRENGTH;
    return table[b.rank] - table[a.rank];
  });
}

/** Position a l'ecran d'un siege, vu depuis le mien : je suis toujours en bas. */
export type Position = "south" | "west" | "north" | "east";

export function positionOf(seat: Seat, mySeat: Seat): Position {
  const order: Position[] = ["south", "west", "north", "east"];
  return order[(seat - mySeat + 4) % 4];
}

export const teamOf = (seat: Seat) => (seat % 2) as 0 | 1;
