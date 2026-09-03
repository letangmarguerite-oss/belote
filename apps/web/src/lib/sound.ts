// Sons et vibrations.
//
// Tout est synthetise a la volee avec l'API Web Audio : aucun fichier a
// telecharger, rien a mettre en cache, et le poids de l'application ne bouge
// pas. Les sons sont courts et discrets — on doit pouvoir jouer une heure sans
// avoir envie de couper.

let context: AudioContext | null = null;

/** Le navigateur refuse de produire du son avant un geste de l'utilisateur. */
function audio(): AudioContext | null {
  if (typeof window === "undefined") return null;
  if (!context) {
    const Ctor =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext })
        .webkitAudioContext;
    if (!Ctor) return null;
    try {
      context = new Ctor();
    } catch {
      return null;
    }
  }
  if (context.state === "suspended") void context.resume();
  return context;
}

/** A appeler au premier clic, pour que le son soit pret ensuite. */
export function primeAudio(): void {
  audio();
}

/** Une note breve, avec une attaque douce et une extinction naturelle. */
function tone(
  freq: number,
  duration: number,
  gain = 0.07,
  type: OscillatorType = "sine",
  delay = 0,
): void {
  const c = audio();
  if (!c) return;

  const osc = c.createOscillator();
  osc.type = type;
  osc.frequency.value = freq;

  const envelope = c.createGain();
  const start = c.currentTime + delay;
  envelope.gain.setValueAtTime(0.0001, start);
  envelope.gain.linearRampToValueAtTime(gain, start + 0.012);
  envelope.gain.exponentialRampToValueAtTime(0.0001, start + duration);

  osc.connect(envelope).connect(c.destination);
  osc.start(start);
  osc.stop(start + duration + 0.03);
}

/**
 * Un souffle bref filtre : c'est ce qui evoque le mieux une carte qu'on pose.
 * Une note pure sonnerait comme un jouet.
 */
function slap(duration: number, freq: number, q: number, gain: number): void {
  const c = audio();
  if (!c) return;

  const length = Math.max(1, Math.floor(c.sampleRate * duration));
  const buffer = c.createBuffer(1, length, c.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i++) {
    // Decroissance quadratique : l'attaque claque, la queue s'efface vite.
    const fade = 1 - i / length;
    data[i] = (Math.random() * 2 - 1) * fade * fade;
  }

  const source = c.createBufferSource();
  source.buffer = buffer;

  const filter = c.createBiquadFilter();
  filter.type = "bandpass";
  filter.frequency.value = freq;
  filter.Q.value = q;

  const envelope = c.createGain();
  envelope.gain.value = gain;

  source.connect(filter).connect(envelope).connect(c.destination);
  source.start();
}

// ---------------------------------------------------------------------------
// Les sons du jeu
// ---------------------------------------------------------------------------

export const sounds = {
  /** Une carte qu'on pose sur le tapis. */
  card: () => slap(0.085, 1700, 1.1, 0.22),

  /** Un pli qu'on ramasse : deux notes qui descendent. */
  trick: () => {
    tone(560, 0.11, 0.05);
    tone(392, 0.17, 0.05, "sine", 0.07);
  },

  /** C'est a vous. Discret, mais on le remarque. */
  turn: () => {
    tone(880, 0.13, 0.055);
    tone(1174, 0.12, 0.045, "sine", 0.09);
  },

  /** Une annonce d'un autre joueur. */
  chat: () => tone(1046, 0.07, 0.045, "triangle"),

  /** Fin de donne : un petit accord ascendant. */
  dealEnd: () => {
    tone(523, 0.16, 0.05);
    tone(659, 0.16, 0.05, "sine", 0.09);
    tone(784, 0.24, 0.05, "sine", 0.18);
  },
};

// ---------------------------------------------------------------------------
// Vibration
// ---------------------------------------------------------------------------

/** Vibration courte. Sans effet sur un ordinateur, et c'est tres bien ainsi. */
export function buzz(pattern: number | number[] = 18): void {
  if (typeof navigator === "undefined" || !("vibrate" in navigator)) return;
  try {
    navigator.vibrate(pattern);
  } catch {
    // Certains navigateurs refusent hors interaction : sans consequence.
  }
}
