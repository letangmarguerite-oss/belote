"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { useAuth } from "@/store/auth";

/**
 * Cadre commun : restaure la session au chargement et affiche l'en-tete.
 *
 * Le jeton d'acces ne survit pas a un rechargement (il est en memoire) ; c'est
 * le cookie de rafraichissement qui rouvre la session, d'ou ce passage
 * obligatoire avant d'afficher quoi que ce soit d'authentifie.
 */
export function Shell({
  children,
  requireAuth = false,
  bare = false,
}: {
  children: React.ReactNode;
  requireAuth?: boolean;
  bare?: boolean;
}) {
  const { user, ready, bootstrap, logout } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (!ready) void bootstrap();
  }, [ready, bootstrap]);

  useEffect(() => {
    if (ready && requireAuth && !user) router.replace("/login");
  }, [ready, requireAuth, user, router]);

  if (!ready) {
    return (
      <main className="flex min-h-dvh items-center justify-center">
        <p className="text-bone-dim">Chargement…</p>
      </main>
    );
  }

  if (requireAuth && !user) return null;

  return (
    <div className="flex min-h-dvh flex-col">
      <header className="flex items-center justify-between gap-3 px-4 py-3">
        <Link href="/" className="font-display text-lg text-gold">
          Belote
        </Link>
        {user && !bare && (
          <div className="flex items-center gap-3 text-sm">
            <Link href="/history" className="text-bone-dim hover:text-bone">
              Historique
            </Link>
            <span className="text-bone-dim">{user.display_name}</span>
            <button
              type="button"
              className="text-bone-dim underline-offset-2 hover:text-bone hover:underline"
              onClick={async () => {
                await logout();
                router.push("/login");
              }}
            >
              Quitter
            </button>
          </div>
        )}
      </header>
      {children}
    </div>
  );
}
