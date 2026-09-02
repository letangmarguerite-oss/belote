// Client WebSocket : ticket a usage unique, reconnexion automatique, battement.
//
// Le socket est volontairement "bete" : il ne connait aucune regle de belote.
// Il transporte des actions et recoit des instantanes. Toute la logique de jeu
// vit cote serveur.

import { API_URL, WS_URL, getAccessToken } from "./api";
import type { ClientMsg, ServerMsg } from "./types";

export type SocketStatus = "connecting" | "open" | "reconnecting" | "closed";

/** Battement : garde la connexion en vie a travers proxys et pare-feux. */
const PING_INTERVAL = 25_000;
const BACKOFF_START = 500;
const BACKOFF_MAX = 10_000;

interface Handlers {
  onMessage: (msg: ServerMsg) => void;
  onStatus: (status: SocketStatus) => void;
}

export class GameSocket {
  private ws: WebSocket | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private backoff = BACKOFF_START;
  private closedByUs = false;
  /** Messages emis pendant une coupure, rejoues a la reconnexion. */
  private queue: ClientMsg[] = [];

  constructor(
    private code: string,
    private handlers: Handlers,
  ) {}

  async connect(): Promise<void> {
    if (this.closedByUs) return;
    this.handlers.onStatus(this.ws ? "reconnecting" : "connecting");

    let ticket: string;
    try {
      // Un ticket ne sert qu'une fois : il en faut un neuf a chaque connexion.
      const res = await fetch(`${API_URL}/api/ws-ticket`, {
        method: "POST",
        credentials: "include",
        headers: getAccessToken()
          ? { authorization: `Bearer ${getAccessToken()}` }
          : {},
      });
      if (!res.ok) throw new Error(`ticket refuse (${res.status})`);
      ticket = ((await res.json()) as { ticket: string }).ticket;
    } catch {
      this.scheduleRetry();
      return;
    }

    const url = `${WS_URL}/ws?ticket=${encodeURIComponent(ticket)}&code=${encodeURIComponent(this.code)}`;
    const ws = new WebSocket(url);
    this.ws = ws;

    ws.addEventListener("open", () => {
      this.backoff = BACKOFF_START;
      this.handlers.onStatus("open");

      // Apres une coupure, l'etat local peut avoir manque des evenements :
      // on redemande l'instantane plutot que de bricoler un rattrapage.
      this.rawSend({ type: "resync" });
      for (const msg of this.queue.splice(0)) this.rawSend(msg);

      this.pingTimer = setInterval(
        () => this.rawSend({ type: "ping" }),
        PING_INTERVAL,
      );
    });

    ws.addEventListener("message", (event) => {
      try {
        this.handlers.onMessage(JSON.parse(event.data as string) as ServerMsg);
      } catch {
        /* message illisible : on l'ignore, l'instantane suivant fait foi */
      }
    });

    ws.addEventListener("close", () => {
      this.clearPing();
      if (!this.closedByUs) this.scheduleRetry();
      else this.handlers.onStatus("closed");
    });

    ws.addEventListener("error", () => ws.close());
  }

  send(msg: ClientMsg): void {
    if (this.ws?.readyState === WebSocket.OPEN) this.rawSend(msg);
    else this.queue.push(msg);
  }

  close(): void {
    this.closedByUs = true;
    this.clearPing();
    if (this.retryTimer) clearTimeout(this.retryTimer);
    this.ws?.close();
    this.handlers.onStatus("closed");
  }

  private rawSend(msg: ClientMsg): void {
    try {
      this.ws?.send(JSON.stringify(msg));
    } catch {
      this.queue.push(msg);
    }
  }

  private clearPing(): void {
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.pingTimer = null;
  }

  private scheduleRetry(): void {
    this.handlers.onStatus("reconnecting");
    if (this.retryTimer) clearTimeout(this.retryTimer);
    const delay = this.backoff;
    // Recul exponentiel : inutile de marteler un serveur qui redemarre.
    this.backoff = Math.min(this.backoff * 2, BACKOFF_MAX);
    this.retryTimer = setTimeout(() => void this.connect(), delay);
  }
}
