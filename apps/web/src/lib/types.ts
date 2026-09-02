// Types du protocole, miroir de belote-core et de belote-server/src/proto.rs.
//
// Ils sont ecrits a la main : le serveur fait autorite, le client ne fait que
// lire. Si un champ manque ici, c'est un affichage en moins, jamais une regle
// de jeu differente — le client ne decide de rien.

export type Suit = "clubs" | "diamonds" | "hearts" | "spades";

export type Rank =
  | "seven"
  | "eight"
  | "nine"
  | "ten"
  | "jack"
  | "queen"
  | "king"
  | "ace";

export interface Card {
  suit: Suit;
  rank: Rank;
}

/** 0 = Sud (moi), 1 = Ouest, 2 = Nord, 3 = Est. Equipes : {0,2} contre {1,3}. */
export type Seat = 0 | 1 | 2 | 3;

export type Phase =
  | "dealing"
  | "bidding1"
  | "bidding2"
  | "playing"
  | "finished"
  | "redeal";

export interface PlayedCard {
  seat: Seat;
  card: Card;
}

export interface DealScore {
  points: [number, number];
  raw: [number, number];
  taker: Seat;
  trump: Suit;
  belote: Seat | null;
  capot: number | null;
  contract_made: boolean;
  litige: boolean;
  carry_out: number;
}

export interface PlayerView {
  seat: Seat;
  phase: Phase;
  turn: Seat;
  dealer: Seat;
  taker: Seat | null;
  trump: Suit | null;
  upcard: Card | null;
  /** Ma main. Celle des autres n'est connue qu'en nombre de cartes. */
  hand: Card[];
  hand_sizes: [number, number, number, number];
  trick: PlayedCard[];
  trick_leader: Seat;
  tricks_played: number;
  tricks_won: [number, number];
  card_points: [number, number];
  belote_mine: boolean;
  /** Ce que je peux poser maintenant. Vide si ce n'est pas mon tour. */
  legal: Card[];
  carry_in: number;
  score: DealScore | null;
}

export interface SeatInfo {
  seat: Seat;
  display_name: string;
  is_bot: boolean;
  connected: boolean;
}

export type PublicEvent =
  | { type: "dealt"; dealer: Seat; hand: Card[]; upcard: Card; hand_sizes: number[]; carry_in: number }
  | { type: "passed"; seat: Seat }
  | { type: "took"; seat: Seat; suit: Suit; from_upcard: boolean }
  | { type: "deal_completed"; extra: Card[]; extra_sizes: number[]; belote_mine: boolean }
  | { type: "redeal" }
  | { type: "belote_shown"; seat: Seat; complete: boolean }
  | { type: "played"; seat: Seat; card: Card }
  | { type: "trick_taken"; winner: Seat; points: number; last: boolean }
  | { type: "scored"; [key: string]: unknown };

export type ServerMsg =
  | { type: "welcome"; seat: Seat; join_code: string; target: number }
  | {
      type: "snapshot";
      seq: number;
      view: PlayerView;
      totals: [number, number];
      carry: number;
      seats: SeatInfo[];
      winner: number | null;
    }
  | { type: "event"; seq: number; event: PublicEvent }
  | { type: "seats"; seats: SeatInfo[] }
  | { type: "error"; message: string }
  | { type: "pong" };

export type Action =
  | { type: "take" }
  | { type: "choose_trump"; suit: Suit }
  | { type: "pass" }
  | { type: "play"; card: Card };

export type ClientMsg =
  | { type: "act"; action: Action }
  | { type: "resync" }
  | { type: "ping" };

// --- Reponses HTTP ---------------------------------------------------------

export interface User {
  id: string;
  email: string;
  display_name: string;
}

export interface AuthResponse {
  access_token: string;
  expires_in: number;
  user: User;
}

export interface TableSeat {
  seat: number;
  user_id: string | null;
  display_name: string | null;
  is_bot: boolean;
}

export interface TableResponse {
  id: string;
  join_code: string;
  status: string;
  owner_id: string;
  seats: TableSeat[];
}

export interface GameSummary {
  id: string;
  table_id: string;
  started_at: string;
  ended_at: string | null;
  final_scores: { totals: [number, number] } | null;
  seat: number;
}
