"use client";

import { useParams, useRouter } from "next/navigation";
import { useEffect } from "react";

import { BidPanel } from "@/components/BidPanel";
import { Hand } from "@/components/Hand";
import { Lobby } from "@/components/Lobby";
import { Scoreboard, TableFelt } from "@/components/TableFelt";
import { Shell } from "@/components/Shell";
import { SUIT_LABEL } from "@/lib/cards";
import type { DealScore, Seat, SeatInfo } from "@/lib/types";
import { useAuth } from "@/store/auth";
import { useGame } from "@/store/game";

export default function TablePage() {
  const params = useParams<{ code: string }>();
  const code = (params.code ?? "").toUpperCase();
  const { user, ready } = useAuth();

  return (
    <Shell requireAuth bare>
      {ready && user ? <Game code={code} /> : null}
    </Shell>
  );
}

function Game({ code }: { code: string }) {
  const router = useRouter();
  const {
    connect,
    disconnect,
    act,
    sendReady,
    sendStart,
    status,
    view,
    seats,
    mySeat,
    totals,
    carry,
    winner,
    ready,
    awaitingContinue,
    inLobby,
    canStart,
    heldTrick,
    flashes,
    error,
    dismissError,
  } = useGame();

  useEffect(() => {
    connect(code);
    return () => disconnect();
  }, [code, connect, disconnect]);

  const leave = () => {
    disconnect();
    router.push("/");
  };

  if (!view || mySeat === null) {
    return (
      <main className="flex flex-1 flex-col items-center justify-center gap-3 px-5">
        <p className="text-bone-dim">
          {status === "reconnecting" ? "Reconnexion…" : "Connexion à la table…"}
        </p>
      </main>
    );
  }

  // Tant que la partie n'est pas lancee, le code reste affiche en grand.
  if (inLobby) {
    return (
      <Lobby
        code={code}
        seats={seats}
        canStart={canStart}
        onStart={sendStart}
        onLeave={leave}
      />
    );
  }

  // La main se verrouille pendant que le pli ramasse est encore affiche :
  // sinon un joueur rapide effacerait de lui-meme ce qu'on lui montre.
  const myTurn =
    view.turn === mySeat && view.phase === "playing" && heldTrick === null;

  return (
    <main className="flex flex-1 flex-col gap-2 px-2 pb-2 sm:px-4">
      {status !== "open" && (
        <Banner tone="warn">
          {status === "reconnecting"
            ? "Connexion perdue, reprise en cours…"
            : "Hors ligne"}
        </Banner>
      )}

      <Scoreboard view={view} totals={totals} mySeat={mySeat} carry={carry} />

      <TableFelt
        view={view}
        seats={seats}
        mySeat={mySeat}
        heldTrick={heldTrick}
      />

      <div className="pointer-events-none flex flex-col items-center gap-1">
        {flashes.map((flash) => (
          <p key={flash.id} className="animate-flash text-sm text-gold">
            {flash.text}
          </p>
        ))}
      </div>

      {view.phase === "bidding1" || view.phase === "bidding2" ? (
        <div className="flex justify-center px-2">
          <BidPanel view={view} onAct={act} />
        </div>
      ) : null}

      {awaitingContinue && view.score && (
        <DealResult
          score={view.score}
          mySeat={mySeat}
          totals={totals}
          winner={winner}
          seats={seats}
          ready={ready}
          onContinue={sendReady}
          onLeave={leave}
        />
      )}

      <Hand
        hand={view.hand}
        legal={view.legal}
        myTurn={myTurn}
        trump={view.trump}
        onPlay={(card) => act({ type: "play", card })}
      />

      {error && (
        <button
          type="button"
          onClick={dismissError}
          className="mx-auto rounded-lg bg-ruby/90 px-4 py-2 text-sm text-bone"
        >
          {error} — toucher pour fermer
        </button>
      )}
    </main>
  );
}

// ---------------------------------------------------------------------------

/**
 * Fin de donne. Rien ne repart tant que les joueurs presents n'ont pas
 * demande la suite : on peut lire le decompte, ou quitter la table.
 */
function DealResult({
  score,
  mySeat,
  totals,
  winner,
  seats,
  ready,
  onContinue,
  onLeave,
}: {
  score: DealScore;
  mySeat: Seat;
  totals: [number, number];
  winner: number | null;
  seats: SeatInfo[];
  ready: Seat[];
  onContinue: () => void;
  onLeave: () => void;
}) {
  const myTeam = mySeat % 2;
  const won = score.points[myTeam] > score.points[1 - myTeam];
  const iAmReady = ready.includes(mySeat);

  // Qui doit encore se prononcer : les joueurs presents, bots exclus.
  const pending = seats.filter(
    (s) => !s.is_bot && s.connected && !ready.includes(s.seat),
  );

  const matchOver = winner !== null;

  return (
    <div className="panel mx-auto flex w-full max-w-md flex-col items-center gap-2 p-4 text-center">
      <p className="font-display text-lg text-gold">
        {score.litige
          ? "Litige"
          : score.contract_made
            ? "Contrat rempli"
            : "Dedans"}
      </p>
      <p className="text-xs text-bone-dim">
        Atout {SUIT_LABEL[score.trump].toLowerCase()}
        {score.capot !== null && " · capot"}
        {score.belote !== null && " · belote"}
      </p>
      <p className="text-2xl">
        <span className={won ? "text-gold" : "text-bone"}>
          {score.points[myTeam]}
        </span>
        <span className="text-bone-dim"> — {score.points[1 - myTeam]}</span>
      </p>
      {score.carry_out > 0 && (
        <p className="text-xs text-bone-dim">
          {score.carry_out} points en cagnotte pour la donne suivante
        </p>
      )}

      {matchOver && (
        <p className="font-display text-lg text-gold">
          {winner === myTeam ? "Vous gagnez le match !" : "Match perdu"} (
          {totals[myTeam]} — {totals[1 - myTeam]})
        </p>
      )}

      <div className="mt-1 flex w-full flex-col gap-2 sm:flex-row sm:justify-center">
        <button
          type="button"
          className="btn btn-gold flex-1"
          onClick={onContinue}
          disabled={iAmReady}
        >
          {iAmReady
            ? "En attente des autres…"
            : matchOver
              ? "Nouveau match"
              : "Donne suivante"}
        </button>
        <button type="button" className="btn btn-ghost flex-1" onClick={onLeave}>
          Quitter la table
        </button>
      </div>

      {iAmReady && pending.length > 0 && (
        <p className="text-xs text-bone-dim">
          On attend {pending.map((s) => s.display_name).join(", ")}
        </p>
      )}
      {!matchOver && (
        <p className="text-[0.7rem] text-bone-dim">
          Sans réponse, la donne suivante démarre au bout d&apos;une minute.
        </p>
      )}
    </div>
  );
}

function Banner({
  children,
  tone,
}: {
  children: React.ReactNode;
  tone: "warn" | "info";
}) {
  return (
    <div
      className={`mx-auto rounded-lg px-3 py-1.5 text-center text-xs ${
        tone === "warn" ? "bg-ruby/25 text-bone" : "bg-ink-900/70 text-bone-dim"
      }`}
    >
      {children}
    </div>
  );
}

