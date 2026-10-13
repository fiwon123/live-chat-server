import { useRef, useState, type FormEvent } from "react";

import { credentialRules, credentialsValid } from "../auth/validation";
import { Icon } from "./Icon";
import { Logo } from "./Logo";

interface LoginFormProps {
  busy: boolean;
  error: string | null;
  onSubmit: (mode: "login" | "register", username: string, password: string) => void;
}

export function LoginForm({ onSubmit, busy, error }: LoginFormProps) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const loginTabRef = useRef<HTMLButtonElement>(null);
  const registerTabRef = useRef<HTMLButtonElement>(null);

  const switchMode = (next: "login" | "register") => {
    if (next === mode) {
      return;
    }
    setMode(next);
    const nextTab = next === "login" ? loginTabRef : registerTabRef;
    window.requestAnimationFrame(() => nextTab.current?.focus());
  };

  const rules = credentialRules(username, password);
  const loginReady = username.length > 0 && password.length > 0;
  const canSubmit = !busy && (mode === "login" ? loginReady : credentialsValid(username, password));

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (!canSubmit) {
      return;
    }
    onSubmit(mode, username, password);
  };

  return (
    <form className="login" onSubmit={handleSubmit} noValidate>
      <header className="login__head">
        <Logo className="login__logo" size={30} withWordmark={false} />
        <h1>{mode === "login" ? "Welcome back" : "Create your account"}</h1>
        <p className="login__sub">
          {mode === "login"
            ? "Log in to jump back into your rooms."
            : "Pick a name and start chatting in seconds."}
        </p>
      </header>

      <div
        className="tabs"
        role="tablist"
        aria-label="Authentication mode"
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
            event.preventDefault();
            switchMode(mode === "login" ? "register" : "login");
          }
        }}
      >
        <span className="tabs__ind" data-mode={mode} aria-hidden="true" />
        <button
          type="button"
          role="tab"
          ref={loginTabRef}
          aria-selected={mode === "login"}
          tabIndex={mode === "login" ? 0 : -1}
          className={mode === "login" ? "active" : ""}
          onClick={() => switchMode("login")}
        >
          Log in
        </button>
        <button
          type="button"
          role="tab"
          ref={registerTabRef}
          aria-selected={mode === "register"}
          tabIndex={mode === "register" ? 0 : -1}
          className={mode === "register" ? "active" : ""}
          onClick={() => switchMode("register")}
        >
          Register
        </button>
      </div>

      <label className="field">
        <span className="field__label">Username</span>
        <span className="field__control">
          <Icon name="user" size={18} className="field__icon" />
          <input
            value={username}
            onChange={(event) => setUsername(event.target.value)}
            autoComplete="username"
            autoCapitalize="none"
            spellCheck={false}
            minLength={3}
            maxLength={32}
            placeholder="username"
            required
          />
        </span>
      </label>

      <label className="field">
        <span className="field__label">Password</span>
        <span className="field__control">
          <Icon name="lock" size={18} className="field__icon" />
          <input
            type={showPassword ? "text" : "password"}
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete={mode === "login" ? "current-password" : "new-password"}
            minLength={8}
            maxLength={128}
            placeholder="••••••••"
            required
          />
          <button
            type="button"
            className="field__reveal"
            onClick={() => setShowPassword((value) => !value)}
            aria-label={showPassword ? "Hide password" : "Show password"}
            aria-pressed={showPassword}
          >
            <Icon name={showPassword ? "eye-off" : "eye"} size={18} />
          </button>
        </span>
      </label>

      {mode === "register" && (
        <ul className="rules">
          {rules.map((rule) => (
            <li key={rule.id} className={rule.met ? "rule rule--met" : "rule"}>
              <Icon name={rule.met ? "check" : "alert"} size={14} />
              {rule.label}
            </li>
          ))}
        </ul>
      )}

      {error !== null && (
        <p className="error error--banner" role="alert">
          <Icon name="alert" size={16} />
          {error}
        </p>
      )}

      <button type="submit" className="btn btn--primary btn--block" disabled={!canSubmit}>
        {busy ? (
          <>
            <Icon name="spinner" size={18} className="icon-spin" />
            Working…
          </>
        ) : mode === "login" ? (
          "Log in"
        ) : (
          "Create account"
        )}
      </button>
    </form>
  );
}
