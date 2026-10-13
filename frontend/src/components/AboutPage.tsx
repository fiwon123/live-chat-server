import { useEffect, useRef } from "react";

import { Icon } from "./Icon";
import { Logo } from "./Logo";

const REPO_URL = "https://github.com/fiwon123/live-chat-server";
const API_DOCS_URL = `${REPO_URL}/blob/main/docs/api.md`;

const STEPS = [
  {
    title: "Create an account",
    text: "Register a username and password, or log back in if you have one.",
  },
  {
    title: "Start or join a room",
    text: "Create a room, or pick one from the list — joining takes a single click.",
  },
  {
    title: "Chat in real time",
    text: "Messages reach everyone in the room instantly over a WebSocket.",
  },
  {
    title: "React to a message",
    text: "Hover a message and tap a quick reaction to reply without typing.",
  },
  {
    title: "Come back later",
    text: "Reconnect any time — your conversation history is saved and still here.",
  },
];

const STACK = [
  "Rust — Axum on Tokio",
  "WebSockets + PostgreSQL (sqlx)",
  "React + TypeScript (Vite)",
  "Serde + tracing",
];

export function AboutPage() {
  const headingRef = useRef<HTMLHeadingElement>(null);

  // Move focus to the page heading on navigation so a route change (which
  // swaps the DOM) is announced to keyboard and screen-reader users.
  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  return (
    <div className="about">
      <header className="about__bar">
        <a className="btn btn--ghost about__back" href="#/">
          <Icon name="chevron-left" size={18} />
          Back to chat
        </a>
        <Logo size={22} />
      </header>

      <div className="about__inner">
        <main className="about__content">
          <section className="about__hero">
            <p className="about__eyebrow">About</p>
            <h1 className="about__title" ref={headingRef} tabIndex={-1}>
              Live chat, minus the setup
            </h1>
            <p className="about__lede">
              Live Chat is a small real-time chat server — Rust on the backend, React in the
              browser, WebSockets in between. Register, create or join a room, and messages land
              instantly, with history that survives a reconnect.
            </p>
            <div className="about__actions">
              <a
                className="btn btn--primary"
                href={REPO_URL}
                target="_blank"
                rel="noopener noreferrer"
              >
                View on GitHub
                <Icon name="external" size={16} />
              </a>
              <a className="btn" href={API_DOCS_URL} target="_blank" rel="noopener noreferrer">
                API reference
                <Icon name="external" size={16} />
              </a>
            </div>
          </section>

          <section className="about__section">
            <h2 className="about__heading">How to use it</h2>
            <ol className="about__steps">
              {STEPS.map((step, index) => (
                <li key={step.title} className="about__step">
                  <span className="about__step-num" aria-hidden="true">
                    {index + 1}
                  </span>
                  <span className="about__step-copy">
                    <strong>{step.title}</strong>
                    <em>{step.text}</em>
                  </span>
                </li>
              ))}
            </ol>
          </section>

          <section className="about__section">
            <h2 className="about__heading">Built with</h2>
            <ul className="about__stack">
              {STACK.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </section>
        </main>

        <footer className="about__footer">
          <a className="about__repo" href={REPO_URL} target="_blank" rel="noopener noreferrer">
            github.com/fiwon123/live-chat-server
          </a>
          <p className="about__fine">
            A personal study and portfolio project by Felipe Inoue. All rights reserved.
          </p>
        </footer>
      </div>
    </div>
  );
}
