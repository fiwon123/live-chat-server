import { useState, type FormEvent } from "react";

interface LoginFormProps {
  busy: boolean;
  error: string | null;
  onSubmit: (mode: "login" | "register", username: string, password: string) => void;
}

export function LoginForm({ onSubmit, busy, error }: LoginFormProps) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit(mode, username, password);
  };

  return (
    <form className="login" onSubmit={handleSubmit}>
      <h1>live-chat</h1>
      <div className="tabs" role="tablist">
        <button
          type="button"
          className={mode === "login" ? "active" : ""}
          onClick={() => setMode("login")}
        >
          Log in
        </button>
        <button
          type="button"
          className={mode === "register" ? "active" : ""}
          onClick={() => setMode("register")}
        >
          Register
        </button>
      </div>

      <label>
        Username
        <input
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          autoComplete="username"
          autoCapitalize="none"
          spellCheck={false}
          minLength={3}
          maxLength={32}
          required
        />
      </label>

      <label>
        Password
        <input
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          autoComplete={mode === "login" ? "current-password" : "new-password"}
          minLength={8}
          maxLength={128}
          required
        />
      </label>

      {error !== null && <p className="error">{error}</p>}

      <button type="submit" disabled={busy}>
        {busy ? "Working…" : mode === "login" ? "Log in" : "Create account"}
      </button>

      <p className="hint">Username: 3–32 chars of a–z, 0–9, _. Password: 8+ chars.</p>
    </form>
  );
}
