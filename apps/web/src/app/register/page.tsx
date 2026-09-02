"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { Field } from "@/components/Field";
import { Shell } from "@/components/Shell";
import { ApiError } from "@/lib/api";
import { useAuth } from "@/store/auth";

export default function RegisterPage() {
  const { register } = useAuth();
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (password.length < 8) {
      setError("Le mot de passe doit faire au moins 8 caractères.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await register(email, password, displayName);
      router.push("/");
    } catch (err) {
      setError(
        err instanceof ApiError ? err.message : "Inscription impossible.",
      );
      setBusy(false);
    }
  };

  return (
    <Shell>
      <main className="mx-auto flex w-full max-w-sm flex-1 flex-col justify-center gap-6 px-5 pb-10">
        <h1 className="font-display text-2xl text-bone">Créer un compte</h1>

        <form onSubmit={submit} className="flex flex-col gap-3">
          <Field
            label="Pseudo"
            type="text"
            value={displayName}
            onChange={setDisplayName}
            autoComplete="nickname"
          />
          <Field
            label="Adresse email"
            type="email"
            value={email}
            onChange={setEmail}
            autoComplete="email"
          />
          <Field
            label="Mot de passe (8 caractères minimum)"
            type="password"
            value={password}
            onChange={setPassword}
            autoComplete="new-password"
          />
          <button type="submit" className="btn btn-gold mt-2 w-full" disabled={busy}>
            {busy ? "…" : "Créer le compte"}
          </button>
        </form>

        {error && <p className="text-sm text-ruby">{error}</p>}

        <p className="text-sm text-bone-dim">
          Déjà inscrit ?{" "}
          <Link href="/login" className="text-gold underline-offset-2 hover:underline">
            Se connecter
          </Link>
        </p>
      </main>
    </Shell>
  );
}
