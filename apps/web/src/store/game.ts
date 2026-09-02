"use client";

import { create } from "zustand";

import { GameSocket, type SocketStatus } from "@/lib/ws";
import type {
  Action,
  PlayedCard,
  PlayerView,
  PublicEvent,
  Seat,
  SeatInfo,
  ServerMsg,
} from "@/lib/types";

/** Message ephemere affiche au-dessus du tapis. */
export interface Flash {
  id: number;
  text: string;
}

/** Un pli ramasse, garde a l'ecran le temps qu'on le voie. */
export interface HeldTrick {
  cards: PlayedCard[];
  winner: Seat;
}

/** Duree d'affichage du pli ramasse. Le serveur observe la meme pause. */
const TRICK_HOLD_MS = 1600;

interface GameState {
  socket: GameSocket | null;
  status: SocketStatus;
  mySeat: Seat | null;
  joinCode: string | null;
  target: number;
  view: PlayerView | null;
  seats: SeatInfo[];
  totals: [number, number];
  carry: number;
  winner: number | null;
  ready: Seat[];
  awaitingContinue: boolean;
  /** Le pli qui vient d'etre ramasse, encore affiche. */
  heldTrick: HeldTrick | null;
  flashes: Flash[];
  error: string | null;

  connect: (code: string) => void;
  disconnect: () => void;
  act: (action: Action) => void;
  sendReady: () => void;
  dismissError: () => void;
}

let flashId = 0;
/** Cartes du pli en cours, reconstituees au fil des evenements. */
let running: PlayedCard[] = [];
let holdTimer: ReturnType<typeof setTimeout> | null = null;

export const useGame = create<GameState>((set, get) => ({
  socket: null,
  status: "closed",
  mySeat: null,
  joinCode: null,
  target: 1000,
  view: null,
  seats: [],
  totals: [0, 0],
  carry: 0,
  winner: null,
  ready: [],
  awaitingContinue: false,
  heldTrick: null,
  flashes: [],
  error: null,

  connect: (code) => {
    get().socket?.close();
    running = [];
    if (holdTimer) clearTimeout(holdTimer);

    const socket = new GameSocket(code, {
      onStatus: (status) => set({ status }),
      onMessage: (msg) => applyMessage(msg, set, get),
    });

    set({ socket, joinCode: code, error: null, heldTrick: null });
    void socket.connect();
  },

  disconnect: () => {
    get().socket?.close();
    if (holdTimer) clearTimeout(holdTimer);
    running = [];
    set({
      socket: null,
      status: "closed",
      view: null,
      mySeat: null,
      seats: [],
      flashes: [],
      heldTrick: null,
    });
  },

  act: (action) => get().socket?.send({ type: "act", action }),

  sendReady: () => get().socket?.send({ type: "ready" }),

  dismissError: () => set({ error: null }),
}));

type Setter = (partial: Partial<GameState>) => void;
type Getter = () => GameState;

function applyMessage(msg: ServerMsg, set: Setter, get: Getter) {
  switch (msg.type) {
    case "welcome":
      set({ mySeat: msg.seat, joinCode: msg.join_code, target: msg.target });
      break;

    case "snapshot":
      // L'instantane fait autorite : on ne fusionne rien, on remplace.
      set({
        view: msg.view,
        totals: msg.totals,
        carry: msg.carry,
        seats: msg.seats,
        winner: msg.winner,
        ready: msg.ready,
        awaitingContinue: msg.awaiting_continue,
      });
      // Un pli est reparti : le precedent n'a plus lieu d'etre affiche.
      if (msg.view.trick.length > 0) {
        running = msg.view.trick;
        clearHold(set);
      }
      break;

    case "seats":
      set({ seats: msg.seats });
      break;

    case "event":
      handleEvent(msg.event, set, get);
      break;

    case "error":
      set({ error: msg.message });
      break;

    case "pong":
      break;
  }
}

function clearHold(set: Setter) {
  if (holdTimer) {
    clearTimeout(holdTimer);
    holdTimer = null;
  }
  set({ heldTrick: null });
}

/** Les evenements servent aux annonces et a l'affichage du pli ramasse. */
function handleEvent(event: PublicEvent, set: Setter, get: Getter) {
  const names = get().seats;
  const nameOf = (seat: Seat) =>
    names.find((s) => s.seat === seat)?.display_name ?? `Siege ${seat}`;

  const flash = (text: string) => {
    const entry = { id: ++flashId, text };
    set({ flashes: [...get().flashes.slice(-2), entry] });
    setTimeout(
      () => set({ flashes: get().flashes.filter((f) => f.id !== entry.id) }),
      3000,
    );
  };

  switch (event.type) {
    case "played":
      // Une nouvelle carte tombe : le pli precedent laisse la place.
      if (get().heldTrick) {
        running = [];
        clearHold(set);
      }
      running = [...running, { seat: event.seat, card: event.card }];
      break;

    case "trick_taken": {
      // Le serveur vide le pli aussitot ramasse. On le garde a l'ecran, sans
      // quoi la quatrieme carte disparaitrait avant d'avoir ete vue — et la
      // derniere du tout n'apparaitrait jamais.
      const cards = running;
      running = [];
      if (cards.length > 0) {
        if (holdTimer) clearTimeout(holdTimer);
        set({ heldTrick: { cards, winner: event.winner } });
        holdTimer = setTimeout(() => {
          holdTimer = null;
          set({ heldTrick: null });
        }, TRICK_HOLD_MS);
      }
      if (event.last) flash("Dix de der");
      break;
    }

    case "dealt":
      running = [];
      clearHold(set);
      break;

    case "passed":
      flash(`${nameOf(event.seat)} passe`);
      break;

    case "took":
      flash(`${nameOf(event.seat)} prend`);
      break;

    case "belote_shown":
      flash(
        `${nameOf(event.seat)} : ${event.complete ? "rebelote" : "belote"} !`,
      );
      break;

    case "redeal":
      flash("Personne ne prend, on redistribue");
      break;

    default:
      break;
  }
}
