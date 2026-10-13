import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { ThemeToggle } from "./ThemeToggle";

// The toggle only differs by its accessible name and icon, derived from the
// current theme; server-render both states so the label can't drift.

const noop = () => {};

describe("ThemeToggle", () => {
  it("invites switching to light while dark", () => {
    const html = renderToStaticMarkup(
      createElement(ThemeToggle, { theme: "dark", onToggle: noop }),
    );
    expect(html).toContain("Switch to light theme");
  });

  it("invites switching to dark while light", () => {
    const html = renderToStaticMarkup(
      createElement(ThemeToggle, { theme: "light", onToggle: noop }),
    );
    expect(html).toContain("Switch to dark theme");
  });
});
