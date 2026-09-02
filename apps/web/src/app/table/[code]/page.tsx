"use client";

import { useParams } from "next/navigation";
import { useEffect, useState } from "react";

import { BidPanel } from "@/components/BidPanel";
import { Hand } from "@/components/Hand";
import { Scoreboard, TableFelt } from "@/components/TableFelt";
import { Shell } from "@/components/Shell";
import { SUIT_LABEL } from "@/lib/cards";
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
  const {
    connect,
    disconnect,
    act,
    status,
    view,
    seats,
    mySeat,
    totals,
    carry,
    winner,
    flashes,
    error,
    dismissError,
  } = useGame();

  useEffect(() => {
    connect(code);
    return () => disconnect();
  }, [code, connect, disconnect]);

  if (!view || mySeat === null) {
    return (
      <main className="flex flex-1 flex-col items-center justify-center gap-3 px-5">
        <p className="text-bone-dim">
          {status === "reconnecting" ? "Reconnexion…" : "Connexion à la table…"}
        </p>
        <ShareCode code={code} />
      </main>
    );
  }

  const myTurn = view.turn === mySeat && view.phase === "playing";
  const waiting = seats.some((s) => !s.is_bot && !s.connected);

  return (
    <main className="flex flex-1 flex-col gap-2 px-2 pb-2 sm:px-4">
      {status !== "open" && (
        <Banner tone="warn">
          {status === "reconnecting"
            ? "Connexion perdue, reprise en cours…"
            : "Hors ligne"}
        </Banner>
      )}

      {waiting && view.phase === "dealing" && (
        <Banner tone="info">
          En attente des autres joueurs. <ShareCode code={code} inline />
        </Banner>
      )}

      <Scoreboard view={view} totals={totals} mySeat={mySeat} carry={carry} />

      <TableFelt view={view} seats={seats} mySeat={mySeat} />

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

      {view.phase === "finished" && view.score && (
        <DealResult
          score={view.score}
          mySeat={mySeat}
          totals={totals}
          winner={winner}
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

function DealResult({
  score,
  mySeat,
  totals,
  winner,
}: {
  score: NonNullable<ReturnType<typeof useGame.getState>["view"]>["score"];
  mySeat: number;
  totals: [number, number];
  winner: number | null;
}) {
  if (!score) return null;
  const myTeam = mySeat % 2;
  const won = score.points[myTeam] > score.points[1 - myTeam];

  return (
    <div className="panel mx-auto flex max-w-md flex-col items-center gap-1 p-4 text-center">
      <p className="font-display text-lg text-gold">
        {score.litige
          ? "Litige"
          : score.contract_made
            ? "Contrat rempli"
            : "Dedans"}
      </p>
      <p className="text-sm text-bone-dim">
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
      {winner !== null ? (
        <p className="mt-1 font-display text-lg text-gold">
          {winner === myTeam ? "Vous gagnez le match !" : "Match perdu"} (
          {totals[myTeam]} — {totals[1 - myTeam]})
        </p>
      ) : (
        <p className="mt-1 text-xs text-bone-dim">Donne suivante dans un instant…</p>
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

/** Le code de la table, copiable d'un geste pour l'envoyer aux amis. */
function ShareCode({ code, inline = false }: { code: string; inline?: boolean }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(
        `${window.location.origin}/table/${code}`,
      );
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Le presse-papiers peut etre refuse : le code reste lisible a l'ecran.
    }
  };

  return (
    <button
      type="button"
      onClick={copy}
      className={
        inline
          ? "font-display tracking-widest text-gold underline-offset-2 hover:underline"
          : "panel px-5 py-3 font-display text-2xl tracking-[0.35em] text-gold"
      }
    >
      {copied ? "Lien copié !" : code}
    </button>
  );
}
