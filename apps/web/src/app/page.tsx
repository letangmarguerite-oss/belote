"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { Shell } from "@/components/Shell";
import { ApiError, api } from "@/lib/api";
import { primeAudio } from "@/lib/sound";
import type { TableResponse } from "@/lib/types";
import { useAuth } from "@/store/auth";

export default function HomePage() {
  const { user } = useAuth();

  return (
    <Shell>
      <main className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center gap-8 px-5 pb-10">
        <div className="text-center">
          <h1 className="font-display text-4xl text-bone">Belote</h1>
          <p className="mt-2 text-sm text-bone-dim">
            Une table, un code, et vos amis vous rejoignent.
          </p>
        </div>

        {user ? <Lobby /> : <SignedOut />}
      </main>
    </Shell>
  );
}

function SignedOut() {
  return (
    <div className="panel flex flex-col gap-3 p-6">
      <p className="text-center text-sm text-bone-dim">
        Connectez-vous pour créer une table.
      </p>
      <Link href="/login" className="btn btn-gold w-full">
        Se connecter
      </Link>
      <Link href="/register" className="btn btn-ghost w-full">
        Créer un compte
      </Link>
    </div>
  );
}

function Lobby() {
  const router = useRouter();
  const [code, setCode] = useState("");
  const [target, setTarget] = useState(1000);
  const [busy, setBusy] = useState<"solo" | "friends" | "join" | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Deux intentions distinctes : la partie solo demarre aussitot, la table
  // entre amis attend au salon le temps de partager son code.
  const create = async (solo: boolean) => {
    // Premier geste de la session : le navigateur autorise le son a partir de
    // la, il faut donc reveiller l'audio maintenant et pas au premier pli.
    primeAudio();
    setBusy(solo ? "solo" : "friends");
    setError(null);
    try {
      const table = await api<TableResponse>(
        `/api/tables?solo=${solo}&target=${target}`,
        { method: "POST" },
      );
      router.push(`/table/${table.join_code}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Création impossible");
      setBusy(null);
    }
  };

  const join = async (event: React.FormEvent) => {
    event.preventDefault();
    const wanted = code.trim().toUpperCase();
    if (wanted.length !== 6) {
      setError("Un code comporte 6 caractères.");
      return;
    }
    primeAudio();
    setBusy("join");
    setError(null);
    try {
      await api(`/api/tables/${wanted}/join`, { method: "POST" });
      router.push(`/table/${wanted}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Impossible de rejoindre");
      setBusy(null);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <button
        type="button"
        className="btn btn-gold w-full flex-col !items-start gap-0 py-3 !min-h-0"
        onClick={() => create(true)}
        disabled={busy !== null}
      >
        <span className="text-base">
          {busy === "solo" ? "Distribution…" : "Jouer seul"}
        </span>
        <span className="text-xs font-normal opacity-75">
          Contre trois bots, la partie commence tout de suite
        </span>
      </button>

      <button
        type="button"
        className="btn btn-ghost w-full flex-col !items-start gap-0 py-3 !min-h-0"
        onClick={() => create(false)}
        disabled={busy !== null}
      >
        <span className="text-base">
          {busy === "friends" ? "Ouverture…" : "Créer une table avec des amis"}
        </span>
        <span className="text-xs font-normal text-bone-dim">
          Vous obtenez un code à partager, et lancez quand tout le monde est là
        </span>
      </button>

      <div className="flex items-center justify-center gap-2 text-xs">
        <span className="text-bone-dim">Partie en</span>
        {[501, 1000, 2000].map((value) => (
          <button
            key={value}
            type="button"
            onClick={() => setTarget(value)}
            className={`rounded-lg px-2.5 py-1 transition-colors ${
              target === value
                ? "bg-gold font-semibold text-ink-950"
                : "bg-ink-900/70 text-bone-dim hover:text-bone"
            }`}
          >
            {value}
          </button>
        ))}
        <span className="text-bone-dim">points</span>
      </div>

      <div className="flex items-center gap-3 text-xs text-bone-dim">
        <span className="h-px flex-1 bg-bone/15" />
        ou rejoindre une table
        <span className="h-px flex-1 bg-bone/15" />
      </div>

      <form onSubmit={join} className="flex gap-2">
        <input
          value={code}
          onChange={(e) => setCode(e.target.value.toUpperCase())}
          placeholder="CODE"
          inputMode="text"
          autoCapitalize="characters"
          autoComplete="off"
          maxLength={6}
          className="panel min-h-11 flex-1 px-4 text-center font-display text-xl tracking-[0.3em] text-bone placeholder:tracking-normal placeholder:text-bone-dim/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-gold"
        />
        <button type="submit" className="btn btn-ghost" disabled={busy !== null}>
          {busy === "join" ? "…" : "Entrer"}
        </button>
      </form>

      {error && <p className="text-center text-sm text-ruby">{error}</p>}

      <p className="text-center text-xs text-bone-dim">
        À une table entre amis, les sièges restés libres sont tenus par des
        bots : pas besoin d&apos;être quatre.
      </p>
    </div>
  );
}
