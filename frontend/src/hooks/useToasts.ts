// Minimal toast system: auto-dismissing status pills.

import { useCallback, useRef, useState } from "react";

export interface Toast {
  id: number;
  kind: "info" | "error";
  text: string;
}

export function useToasts(dismissMs = 3200) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);
  const timers = useRef(new Map<number, number>());

  const dismiss = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
    const timer = timers.current.get(id);
    if (timer !== undefined) {
      window.clearTimeout(timer);
      timers.current.delete(id);
    }
  }, []);

  const push = useCallback(
    (kind: Toast["kind"], text: string) => {
      const id = nextId.current;
      nextId.current += 1;
      setToasts((current) => [...current.slice(-3), { id, kind, text }]);
      timers.current.set(
        id,
        window.setTimeout(() => dismiss(id), dismissMs),
      );
    },
    [dismiss, dismissMs],
  );

  return { toasts, push, dismiss };
}
