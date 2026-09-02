"use client";

import { CardBack, PlayingCard } from "./PlayingCard";
import {
  SUIT_LABEL,
  SUIT_SYMBOL,
  isRed,
  positionOf,
  type Position,
} from "@/lib/cards";
import type { PlayerView, Seat, SeatInfo, Suit } from "@/lib/types";
import type { HeldTrick } from "@/store/game";

interface Props {
  view: PlayerView;
  seats: SeatInfo[];
  mySeat: Seat;
  /** Pli ramasse encore affiche, le temps qu'on le voie. */
  heldTrick: HeldTrick | null;
}

/** Le tapis : les quatre joueurs autour, le pli en cours au centre. */
export function TableFelt({ view, seats, mySeat, heldTrick }: Props) {
  const at = (position: Position) =>
    ([0, 1, 2, 3] as Seat[]).find((s) => positionOf(s, mySeat) === position)!;

  return (
    <div className="relative mx-auto w-full max-w-3xl flex-1">
      <div className="grid h-full grid-cols-[minmax(0,1fr)_minmax(0,2fr)_minmax(0,1fr)] grid-rows-[auto_minmax(0,1fr)_auto] gap-1 sm:gap-3">
        <div className="col-span-3 flex justify-center">
          <SeatPlate seat={at("north")} view={view} seats={seats} layout="row" />
        </div>

        <div className="flex items-center justify-start">
          <SeatPlate seat={at("west")} view={view} seats={seats} layout="column" />
        </div>

        <Trick view={view} mySeat={mySeat} heldTrick={heldTrick} />

        <div className="flex items-center justify-end">
          <SeatPlate seat={at("east")} view={view} seats={seats} layout="column" />
        </div>

        {/* Mon propre bandeau : sans lui je ne saurais pas que je suis
            preneur ou donneur, alors que je le vois pour les autres. */}
        <div className="col-span-3 flex justify-center">
          <SeatPlate seat={mySeat} view={view} seats={seats} layout="none" />
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------

/**
 * Le jeton d'atout, pose devant le preneur : on voit d'un coup d'oeil qui a
 * pris et a quelle couleur, sans avoir a s'en souvenir.
 */
function TrumpToken({ suit }: { suit: Suit }) {
  return (
    <span
      className="inline-flex size-6 items-center justify-center rounded-[0.35rem] border border-gold bg-ink-950 text-sm leading-none shadow-[0_1px_3px_rgba(0,0,0,0.6)]"
      style={{ color: isRed(suit) ? "var(--color-ruby)" : "var(--color-bone)" }}
      title={`Preneur — atout ${SUIT_LABEL[suit].toLowerCase()}`}
      aria-label={`preneur, atout ${SUIT_LABEL[suit].toLowerCase()}`}
    >
      {SUIT_SYMBOL[suit]}
    </span>
  );
}

/** Le jeton de donneur, qui tourne d'un siege a chaque donne. */
function DealerChip() {
  return (
    <span
      className="inline-flex size-5 items-center justify-center rounded-full border border-bone/45 text-[0.6rem] font-bold text-bone-dim"
      title="Donneur"
      aria-label="donneur"
    >
      D
    </span>
  );
}

function SeatPlate({
  seat,
  view,
  seats,
  layout,
}: {
  seat: Seat;
  view: PlayerView;
  seats: SeatInfo[];
  /** "none" pour mon siege : ma main est affichee en dessous. */
  layout: "row" | "column" | "none";
}) {
  const info = seats.find((s) => s.seat === seat);
  const count = view.hand_sizes[seat];
  const isTurn = view.turn === seat && view.phase !== "finished";
  const isTaker = view.taker === seat;
  const isDealer = view.dealer === seat;

  return (
    <div className="flex flex-col items-center gap-1.5">
      <div
        className={`flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs transition-colors sm:text-sm ${
          isTurn
            ? "bg-gold font-semibold text-ink-950"
            : "bg-ink-900/70 text-bone-dim"
        }`}
      >
        <span
          className={`inline-block size-1.5 rounded-full ${
            info?.is_bot
              ? "bg-bone-dim"
              : info?.connected
                ? "bg-emerald-400"
                : "bg-ruby"
          }`}
          title={
            info?.is_bot
              ? "Bot"
              : info?.connected
                ? "Connecté"
                : "Déconnecté, un bot le remplace"
          }
        />
        <span className="max-w-28 truncate">{info?.display_name ?? "…"}</span>
        {isDealer && <DealerChip />}
        {isTaker && view.trump && <TrumpToken suit={view.trump} />}
      </div>

      {layout !== "none" && (
        <div
          className={`flex ${layout === "column" ? "flex-col" : "flex-row"}`}
          aria-label={`${count} cartes`}
        >
          {Array.from({ length: count }).map((_, i) => (
            <div
              key={i}
              className="shrink-0"
              style={{
                width: layout === "column" ? "1.6rem" : "1.5rem",
                aspectRatio: "100 / 150",
                marginLeft: layout === "row" && i > 0 ? "-0.85rem" : 0,
                marginTop: layout === "column" && i > 0 ? "-1.55rem" : 0,
              }}
            >
              <CardBack />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** Le pli en cours, chaque carte placee du cote de qui l'a posee. */
function Trick({
  view,
  mySeat,
  heldTrick,
}: {
  view: PlayerView;
  mySeat: Seat;
  heldTrick: HeldTrick | null;
}) {
  const slot: Record<Position, string> = {
    south: "bottom-0 left-1/2 -translate-x-1/2",
    west: "left-0 top-1/2 -translate-y-1/2",
    north: "top-0 left-1/2 -translate-x-1/2",
    east: "right-0 top-1/2 -translate-y-1/2",
  };

  // Le pli ramasse prend le pas sur le tapis vide renvoye par le serveur.
  const cards = heldTrick ? heldTrick.cards : view.trick;
  const showUpcard =
    cards.length === 0 && view.upcard && view.phase !== "playing";

  return (
    <div className="relative flex min-h-40 items-center justify-center sm:min-h-56">
      <div className="pointer-events-none absolute inset-3 rounded-[45%] border border-gold/15" />

      {showUpcard && view.upcard && (
        <div className="flex flex-col items-center gap-1">
          <div className="w-16 sm:w-20" style={{ aspectRatio: "100 / 150" }}>
            <PlayingCard card={view.upcard} />
          </div>
          <span className="text-[0.7rem] text-bone-dim">carte retournée</span>
        </div>
      )}

      {cards.map((played) => {
        const wins = heldTrick?.winner === played.seat;
        return (
          <div
            // La carte identifie l'entree mieux que le siege : elle reste
            // unique meme si un message arrive en double.
            key={`${played.seat}-${played.card.rank}-${played.card.suit}`}
            className={`animate-deal absolute w-14 transition-transform duration-300 sm:w-16 ${slot[positionOf(played.seat, mySeat)]}`}
            style={{
              aspectRatio: "100 / 150",
              // La carte gagnante se detache pendant que le pli est ramasse.
              transform: wins ? "scale(1.12)" : undefined,
              filter: wins
                ? "drop-shadow(0 0 10px var(--color-gold))"
                : heldTrick
                  ? "brightness(0.72)"
                  : undefined,
              zIndex: wins ? 10 : 1,
            }}
          >
            <PlayingCard card={played.card} />
          </div>
        );
      })}
    </div>
  );
}

/**
 * Bandeau de score.
 *
 * Les points de la donne en cours n'y figurent pas : a la belote on les
 * compte en ramassant, pas en cours de route. Le decompte s'affiche a la fin
 * de la donne, dans le panneau de resultat.
 */
export function Scoreboard({
  view,
  totals,
  mySeat,
  carry,
}: {
  view: PlayerView;
  totals: [number, number];
  mySeat: Seat;
  carry: number;
}) {
  const myTeam = (mySeat % 2) as 0 | 1;
  const them = (1 - myTeam) as 0 | 1;

  return (
    <div className="flex flex-wrap items-center justify-center gap-x-4 gap-y-1 text-xs sm:text-sm">
      <span className="text-bone-dim">
        Plis <span className="text-bone">{view.tricks_won[myTeam]}</span> —{" "}
        <span className="text-bone">{view.tricks_won[them]}</span>
      </span>
      <span className="text-gold">
        Match {totals[myTeam]} — {totals[them]}
      </span>
      {carry > 0 && (
        <span className="text-bone-dim" title="Cagnotte issue d'un litige">
          cagnotte {carry}
        </span>
      )}
    </div>
  );
}
