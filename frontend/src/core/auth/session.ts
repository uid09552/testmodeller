import { Injectable } from '@angular/core';

/**
 * Where the gateway ends the session (FR-046).
 *
 * APISIX's `openid-connect` plugin owns this path; `gateway/apisix.yaml` sets
 * `logout_path` to the same value.
 */
export const LOGOUT_PATH = '/logout';

/**
 * The browser's side of authentication.
 *
 * There is nothing to store: the gateway performs the OIDC handshake and keeps
 * the session in a cookie, so the SPA never holds a token (FR-048, ADR 0005).
 * That leaves exactly two things to do — end the session, and recover when it
 * has already ended.
 */
@Injectable({ providedIn: 'root' })
export class SessionService {
  /**
   * Signs out by navigating to the gateway's logout path.
   *
   * A full navigation, not a router navigation: the session is a gateway
   * cookie, and nothing the SPA clears would end it.
   */
  signOut(): void {
    window.location.assign(LOGOUT_PATH);
  }

  /**
   * Recovers from an expired session by reloading, which sends the browser
   * back through the gateway's handshake.
   *
   * Reloading on a 401 could loop if the gateway keeps rejecting us, so it
   * happens at most once per `COOLDOWN_MS`; after that the error surfaces to
   * the caller instead.
   */
  reauthenticate(): boolean {
    const last = Number(read(RELOAD_KEY) ?? 0);
    if (Date.now() - last < COOLDOWN_MS) return false;
    write(RELOAD_KEY, String(Date.now()));
    window.location.reload();
    return true;
  }
}

const RELOAD_KEY = 'tm:auth-reload-at';
const COOLDOWN_MS = 60_000;

// Storage throws in private windows and when site data is blocked, and a
// failure here must not stop the error from being reported.
function read(key: string): string | null {
  try {
    return sessionStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    sessionStorage.setItem(key, value);
  } catch {
    /* ignore */
  }
}
