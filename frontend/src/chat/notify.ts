// Browser notifications for incoming messages (used only while the tab is
// hidden). Everything degrades gracefully when unsupported/denied.

/** Ask for permission; safe to call repeatedly, resolves immediately if decided. */
export async function requestNotificationPermission(): Promise<void> {
  if (typeof window === "undefined" || !("Notification" in window)) {
    return;
  }
  if (Notification.permission === "default") {
    try {
      await Notification.requestPermission();
    } catch {
      // Some browsers reject the promise form; ignore.
    }
  }
}

export function notificationsEnabled(): boolean {
  return (
    typeof window !== "undefined" &&
    "Notification" in window &&
    Notification.permission === "granted"
  );
}

export function notifyNewMessage(title: string, body: string): void {
  if (!notificationsEnabled() || document.visibilityState === "visible") {
    return;
  }
  try {
    const notification = new Notification(title, {
      body,
      icon: "/favicon.svg",
      tag: "livechat-message",
    });
    notification.onclick = () => {
      window.focus();
      notification.close();
    };
  } catch {
    // Never let a notification failure crash the app.
  }
}
