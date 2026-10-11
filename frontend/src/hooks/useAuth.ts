// Authentication state: token persistence, validation on load, login/register/logout.

import { useCallback, useEffect, useState } from "react";

import { api } from "../api/endpoints";
import { messageOf } from "../api/client";
import type { User } from "../api/types";

const TOKEN_KEY = "live-chat.token";

export type AuthStatus = "loading" | "anonymous" | "authenticated";

export interface Auth {
  status: AuthStatus;
  token: string | null;
  user: User | null;
  error: string | null;
  busy: boolean;
  authenticate: (mode: "login" | "register", username: string, password: string) => Promise<void>;
  logout: () => void;
}

function readToken(): string | null {
  try {
    return window.localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

function writeToken(token: string | null): void {
  try {
    if (token) {
      window.localStorage.setItem(TOKEN_KEY, token);
    } else {
      window.localStorage.removeItem(TOKEN_KEY);
    }
  } catch {
    // Storage unavailable (private mode); the session simply won't persist.
  }
}

export function useAuth(): Auth {
  const [status, setStatus] = useState<AuthStatus>("loading");
  const [token, setToken] = useState<string | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Validate a stored token once on mount.
  useEffect(() => {
    const stored = readToken();
    if (!stored) {
      setStatus("anonymous");
      return;
    }
    let cancelled = false;
    api
      .me(stored)
      .then((me) => {
        if (cancelled) {
          return;
        }
        setToken(stored);
        setUser(me);
        setStatus("authenticated");
      })
      .catch(() => {
        if (cancelled) {
          return;
        }
        writeToken(null);
        setStatus("anonymous");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const authenticate = useCallback(
    async (mode: "login" | "register", username: string, password: string) => {
      setBusy(true);
      setError(null);
      try {
        const result =
          mode === "login"
            ? await api.login(username, password)
            : await api.register(username, password);
        writeToken(result.token);
        setToken(result.token);
        setUser(result.user);
        setStatus("authenticated");
      } catch (caught) {
        writeToken(null);
        setToken(null);
        setUser(null);
        setError(messageOf(caught));
        setStatus("anonymous");
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  const logout = useCallback(() => {
    writeToken(null);
    setToken(null);
    setUser(null);
    setError(null);
    setStatus("anonymous");
  }, []);

  return { status, token, user, error, busy, authenticate, logout };
}
