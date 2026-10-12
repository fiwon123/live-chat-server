// Web Audio chime for incoming messages. Contexts are created lazily and only
// after a user gesture, so a blocked autoplay policy never throws.

let context: AudioContext | null = null;

function getContext(): AudioContext | null {
  if (typeof window === "undefined" || typeof AudioContext === "undefined") {
    return null;
  }
  if (context === null) {
    try {
      context = new AudioContext();
    } catch {
      return null;
    }
  }
  return context;
}

/** Call from a user gesture (login, click, keydown) to satisfy autoplay. */
export function unlockAudio(): void {
  const audio = getContext();
  if (audio && audio.state === "suspended") {
    void audio.resume();
  }
}

/** Short two-note chime; silently no-ops where audio is unavailable. */
export function playChime(): void {
  const audio = getContext();
  if (!audio) {
    return;
  }
  if (audio.state === "suspended") {
    return;
  }
  const now = audio.currentTime;
  for (const [frequency, offset] of [
    [880, 0],
    [1174.66, 0.08],
  ]) {
    const oscillator = audio.createOscillator();
    const gain = audio.createGain();
    oscillator.connect(gain);
    gain.connect(audio.destination);
    oscillator.type = "sine";
    oscillator.frequency.value = frequency;
    gain.gain.setValueAtTime(0.0001, now + offset);
    gain.gain.exponentialRampToValueAtTime(0.12, now + offset + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + offset + 0.28);
    oscillator.start(now + offset);
    oscillator.stop(now + offset + 0.3);
  }
}
