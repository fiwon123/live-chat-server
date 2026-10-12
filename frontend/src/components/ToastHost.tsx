import type { Toast } from "../hooks/useToasts";
import { Icon } from "./Icon";

interface ToastHostProps {
  toasts: Toast[];
  onDismiss: (id: number) => void;
}

export function ToastHost({ toasts, onDismiss }: ToastHostProps) {
  if (toasts.length === 0) {
    return null;
  }
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className={`toast toast--${toast.kind}`}>
          <Icon name={toast.kind === "error" ? "alert" : "check"} size={15} />
          <span>{toast.text}</span>
          <button
            type="button"
            className="toast__close"
            onClick={() => onDismiss(toast.id)}
            aria-label="Dismiss notification"
          >
            <Icon name="close" size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}
