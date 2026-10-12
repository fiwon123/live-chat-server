import { useEffect, useId, useRef, useState } from "react";

import { EMOJIS } from "../chat/emojis";
import { Icon } from "./Icon";

interface EmojiPickerProps {
  onPick: (emoji: string) => void;
  disabled?: boolean;
}

export function EmojiPicker({ onPick, disabled }: EmojiPickerProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const popId = useId();

  useEffect(() => {
    if (!open) {
      return;
    }
    const onPointerDown = (event: PointerEvent) => {
      if (rootRef.current && !rootRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  return (
    <div className="emoji" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        className="btn btn--ghost btn--icon composer__emoji"
        onClick={() => setOpen((current) => !current)}
        disabled={disabled}
        aria-label="Insert emoji"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-controls={open ? popId : undefined}
        title="Emoji"
      >
        <Icon name="smile" size={18} />
      </button>
      {open && (
        <div className="emoji__pop" id={popId} role="dialog" aria-label="Emoji picker">
          {EMOJIS.map((entry) => (
            <button
              key={entry.emoji}
              type="button"
              className="emoji__item"
              onClick={() => {
                onPick(entry.emoji);
                setOpen(false);
              }}
              aria-label={entry.name}
              title={entry.name}
            >
              {entry.emoji}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
