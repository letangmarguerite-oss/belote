"use client";

import { create } from "zustand";

import { GameSocket, type SocketStatus } from "@/lib/ws";
import type {
  Action,
  Card,
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
  /** Derniere carte posee, pour l'animation. */
  lastPlayed: Card | null;
  flashes: Flash[];
  error: string | null;

  connect: (code: string) => void;
  disconnect: () => void;
  act: (action: Action) => void;
  dismissError: () => void;
}

let flashId = 0;

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
  lastPlayed: null,
  flashes: [],
  error: null,

  connect: (code) => {
    get().socket?.close();

    const socket = new GameSocket(code, {
      onStatus: (status) => set({ status }),
      onMessage: (msg) => applyMessage(msg, set, get),
    });

    set({ socket, joinCode: code, error: null });
    void socket.connect();
  },

  disconnect: () => {
    get().socket?.close();
    set({
      socket: null,
      status: "closed",
      view: null,
      mySeat: null,
      seats: [],
      flashes: [],
    });
  },

  act: (action) => get().socket?.send({ type: "act", action }),

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
      // L'instantane fait autorite : on ne "fusionne" rien, on remplace.
      set({
        view: msg.view,
        totals: msg.totals,
        carry: msg.carry,
        seats: msg.seats,
        winner: msg.winner,
      });
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

/** Les evenements ne servent qu'a l'agrement : animations et annonces. */
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
      set({ lastPlayed: event.card });
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
    case "trick_taken":
      if (event.last) flash("Dix de der");
      break;
    default:
      break;
  }
}
