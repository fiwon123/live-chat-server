import { Icon, type IconName } from "./Icon";
import { LoginForm } from "./LoginForm";
import { Logo } from "./Logo";

interface AuthScreenProps {
  busy: boolean;
  error: string | null;
  onSubmit: (mode: "login" | "register", username: string, password: string) => void;
}

const FEATURES: { icon: IconName; title: string; text: string }[] = [
  {
    icon: "message",
    title: "Real-time rooms",
    text: "Messages land instantly over WebSocket.",
  },
  {
    icon: "arrow-down",
    title: "Persistent history",
    text: "Reconnect and your messages are still here.",
  },
  {
    icon: "bell",
    title: "Presence at a glance",
    text: "See who is online in every room.",
  },
];

export function AuthScreen({ busy, error, onSubmit }: AuthScreenProps) {
  return (
    <main className="auth">
      <a className="auth__about" href="#/about">
        <Icon name="info" size={15} />
        About this project
      </a>
      <section className="auth__brand">
        <div className="auth__glow" aria-hidden="true" />
        <Logo size={32} />
        <h2 className="auth__tagline">
          Real-time rooms,
          <br />
          zero refresh.
        </h2>
        <p className="auth__blurb">A tiny, fast chat server built on Rust and WebSockets.</p>

        <ul className="auth__features">
          {FEATURES.map((feature) => (
            <li key={feature.title}>
              <span className="auth__feature-icon">
                <Icon name={feature.icon} size={18} />
              </span>
              <span className="auth__feature-copy">
                <strong>{feature.title}</strong>
                <em>{feature.text}</em>
              </span>
            </li>
          ))}
        </ul>
      </section>

      <section className="auth__panel">
        <LoginForm busy={busy} error={error} onSubmit={onSubmit} />
      </section>
    </main>
  );
}
