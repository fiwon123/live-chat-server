// Tiny hash router. The app has exactly two views — chat and About — so a full
// router dependency would be overkill. `useSyncExternalStore` keeps components
// in sync with `location.hash`, including the browser back/forward buttons.

import { useSyncExternalStore } from "react";

export type Route = "chat" | "about";

/** Map a `location.hash` to a route: `#/about` is About, everything else chat. */
export function routeFromHash(hash: string): Route {
  return hash.replace(/^#\/?/, "").replace(/\/+$/, "").toLowerCase() === "about" ? "about" : "chat";
}

function subscribe(onChange: () => void): () => void {
  window.addEventListener("hashchange", onChange);
  return () => window.removeEventListener("hashchange", onChange);
}

function readRoute(): Route {
  return routeFromHash(window.location.hash);
}

/** The current hash route, re-rendering on `hashchange`. */
export function useHashRoute(): Route {
  return useSyncExternalStore(subscribe, readRoute, () => "chat");
}
