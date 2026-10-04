/**
 * Typed localStorage wrapper.
 *
 * Interim persistence while the HTTP API is being wired up: the backend owns
 * real storage (see docs/specification), so this exists only so work is not
 * lost across a page refresh during development. Every access is guarded —
 * storage throws in private windows and when site data is blocked.
 */

const PREFIX = 'tm:';

export function readJson<T>(key: string): T | null {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    return raw === null ? null : (JSON.parse(raw) as T);
  } catch {
    return null;
  }
}

export function writeJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify(value));
  } catch {
    // Quota exceeded or storage unavailable — nothing useful to do here.
  }
}

export function removeKey(key: string): void {
  try {
    localStorage.removeItem(PREFIX + key);
  } catch {
    /* ignore */
  }
}
