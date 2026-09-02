"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { Field } from "@/components/Field";
import { Shell } from "@/components/Shell";
import { ApiError } from "@/lib/api";
import { useAuth } from "@/store/auth";

export default function LoginPage() {
  const { login } = useAuth();
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await login(email, password);
      router.push("/");
    } catch (err) {
      // Le serveur repond pareil pour un compte inconnu et un mauvais mot de
      // passe : on garde ce flou ici aussi.
      setError(
        err instanceof ApiError && err.status === 401
          ? "Adresse ou mot de passe incorrect."
          : "Connexion impossible.",
      );
      setBusy(false);
    }
  };

  return (
    <Shell>
      <main className="mx-auto flex w-full max-w-sm flex-1 flex-col justify-center gap-6 px-5 pb-10">
        <h1 className="font-display text-2xl text-bone">Se connecter</h1>

        <form onSubmit={submit} className="flex flex-col gap-3">
          <Field
            label="Adresse email"
            type="email"
            value={email}
            onChange={setEmail}
            autoComplete="email"
          />
          <Field
            label="Mot de passe"
            type="password"
            value={password}
            onChange={setPassword}
            autoComplete="current-password"
          />
          <button type="submit" className="btn btn-gold mt-2 w-full" disabled={busy}>
            {busy ? "…" : "Entrer"}
          </button>
        </form>

        {error && <p className="text-sm text-ruby">{error}</p>}

        <p className="text-sm text-bone-dim">
          Pas encore de compte ?{" "}
          <Link href="/register" className="text-gold underline-offset-2 hover:underline">
            En créer un
          </Link>
        </p>
      </main>
    </Shell>
  );
}

