"use client";

import { create } from "zustand";

import { buzz, sounds } from "@/lib/sound";
import { GameSocket, type SocketStatus } from "@/lib/ws";
import { useSettings } from "./settings";
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
/** Duree d'affichage d'une bulle d'annonce. */
const SAY_HOLD_MS = 3200;

/** Joue un son si le joueur les a laisses actifs. */
function chime(play: () => void) {
  if (useSettings.getState().sound) play();
}

/** Vibre si le joueur l'a laisse actif. Sans effet sur un ordinateur. */
function haptic(pattern?: number | number[]) {
  if (useSettings.getState().vibrate) buzz(pattern);
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
  ready: Seat[];
  awaitingContinue: boolean;
  inLobby: boolean;
  canStart: boolean;
  /** Le pli qui vient d'etre ramasse, encore affiche. */
  heldTrick: HeldTrick | null;
  /** Annonce en cours d'affichage pour chaque siege. */
  says: Partial<Record<Seat, { phrase: number; id: number }>>;
  flashes: Flash[];
  error: string | null;

  connect: (code: string) => void;
  disconnect: () => void;
  act: (action: Action) => void;
  sendReady: () => void;
  sendStart: () => void;
  say: (phrase: number) => void;
  dismissError: () => void;
}

let flashId = 0;
/** Pour ne sonner qu'au moment ou le tour arrive, pas a chaque instantane. */
let wasMyTurn = false;
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
  inLobby: true,
  canStart: false,
  heldTrick: null,
  says: {},
  flashes: [],
  error: null,

  connect: (code) => {
    get().socket?.close();
    running = [];
    if (holdTimer) clearTimeout(holdTimer);

    // `socket` est capture pour se comparer plus bas : un socket remplace ne
    // doit plus toucher a l'etat, sous peine de dedoubler chaque carte posee.
    const socket: GameSocket = new GameSocket(code, {
      onStatus: (status) => {
        if (get().socket === socket) set({ status });
      },
      onMessage: (msg) => {
        if (get().socket !== socket) return;
        applyMessage(msg, set, get);
      },
    });

    wasMyTurn = false;
    set({ socket, joinCode: code, error: null, heldTrick: null, says: {} });
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
      says: {},
    });
  },

  act: (action) => get().socket?.send({ type: "act", action }),

  sendReady: () => get().socket?.send({ type: "ready" }),

  sendStart: () => get().socket?.send({ type: "start" }),

  say: (phrase) => get().socket?.send({ type: "say", phrase }),

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
        inLobby: msg.in_lobby,
        canStart: msg.can_start,
        joinCode: msg.join_code,
      });
      // Un pli est reparti : le precedent n'a plus lieu d'etre affiche.
      if (msg.view.trick.length > 0) {
        running = msg.view.trick;
        clearHold(set);
      }

      // Signaler l'arrivee du tour, une seule fois : un instantane est renvoye
      // a chaque coup, y compris pendant que c'est deja a moi.
      {
        const mine =
          msg.view.turn === msg.view.seat &&
          (msg.view.phase === "playing" ||
            msg.view.phase === "bidding1" ||
            msg.view.phase === "bidding2");
        if (mine && !wasMyTurn) {
          chime(sounds.turn);
          haptic();
        }
        wasMyTurn = mine;
      }
      break;

    case "seats":
      set({ seats: msg.seats });
      break;

    case "said": {
      const id = ++flashId;
      set({ says: { ...get().says, [msg.seat]: { phrase: msg.phrase, id } } });
      // Ma propre annonce ne me surprend pas : pas de son pour elle.
      if (msg.seat !== get().mySeat) chime(sounds.chat);
      setTimeout(() => {
        const current = get().says[msg.seat];
        if (current?.id !== id) return; // une annonce plus recente l'a remplacee
        const next = { ...get().says };
        delete next[msg.seat];
        set({ says: next });
      }, SAY_HOLD_MS);
      break;
    }

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
      // Un siege ne pose qu'une carte par pli : si on le revoit, c'est un
      // doublon (message rejoue apres une reconnexion, par exemple).
      running = [
        ...running.filter((p) => p.seat !== event.seat),
        { seat: event.seat, card: event.card },
      ];
      chime(sounds.card);
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
      chime(sounds.trick);
      if (event.last) flash("Dix de der");
      break;
    }

    case "dealt": {
      running = [];
      clearHold(set);
      // Le donneur et l'entameur sont l'information la plus utile a cet
      // instant precis, et la plus vite oubliee ensuite.
      const first = ((event.dealer + 1) % 4) as Seat;
      flash(`${nameOf(event.dealer)} distribue · ${nameOf(first)} commence`);
      break;
    }

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

    case "scored":
      chime(sounds.dealEnd);
      haptic([12, 60, 12]);
      break;

    default:
      break;
  }
}
