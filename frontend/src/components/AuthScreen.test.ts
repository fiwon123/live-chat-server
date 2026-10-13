import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AuthScreen } from "./AuthScreen";

// DOM-free smoke test: the auth screen is otherwise only exercised through the
// browser, so server-rendering it catches a crash and pins the corner controls
// added with the theme toggle.

const noop = () => {};

describe("AuthScreen", () => {
  const html = renderToStaticMarkup(
    createElement(AuthScreen, {
      busy: false,
      error: null,
      onSubmit: noop,
      theme: "dark",
      onToggleTheme: noop,
    }),
  );

  it("renders the login form", () => {
    expect(html).toContain("Welcome back");
  });

  it("offers a theme toggle and an About link in the corner", () => {
    expect(html).toContain("auth__corner");
    expect(html).toContain("Switch to light theme");
    expect(html).toContain('href="#/about"');
  });
});
