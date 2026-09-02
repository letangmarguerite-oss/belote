"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { Shell } from "@/components/Shell";
import { ApiError, api } from "@/lib/api";
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
  const [busy, setBusy] = useState<"create" | "join" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const create = async () => {
    setBusy("create");
    setError(null);
    try {
      const table = await api<TableResponse>("/api/tables", { method: "POST" });
      router.push(`/table/${table.join_code}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Création impossible");
      setBusy(null);
    }
  };

  const join = async (event: React.FormEvent) => {
    event.preventDefault();
    const target = code.trim().toUpperCase();
    if (target.length !== 6) {
      setError("Un code comporte 6 caractères.");
      return;
    }
    setBusy("join");
    setError(null);
    try {
      await api(`/api/tables/${target}/join`, { method: "POST" });
      router.push(`/table/${target}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Impossible de rejoindre");
      setBusy(null);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <button
        type="button"
        className="btn btn-gold w-full"
        onClick={create}
        disabled={busy !== null}
      >
        {busy === "create" ? "Création…" : "Créer une table"}
      </button>

      <div className="flex items-center gap-3 text-xs text-bone-dim">
        <span className="h-px flex-1 bg-bone/15" />
        ou rejoindre
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
        Les sièges vides sont tenus par des bots : vous pouvez jouer même à un
        seul joueur.
      </p>
    </div>
  );
}
