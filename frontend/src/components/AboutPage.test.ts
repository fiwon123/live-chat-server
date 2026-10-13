import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AboutPage } from "./AboutPage";

// A DOM-free smoke test: the About page is static, so server-rendering it
// catches a crash and pins the content the acceptance criteria call for.

const noop = () => {};

describe("AboutPage", () => {
  const html = renderToStaticMarkup(
    createElement(AboutPage, { theme: "dark", onToggleTheme: noop }),
  );

  it("links the repository", () => {
    expect(html).toContain("https://github.com/fiwon123/live-chat-server");
  });

  it("walks through how to use the app", () => {
    expect(html).toContain("How to use it");
    expect(html).toContain("Create an account");
    expect(html).toContain("Come back later");
  });

  it("links the API reference", () => {
    expect(html).toContain("docs/api.md");
  });

  it("offers a way back to the app", () => {
    expect(html).toContain('href="#/"');
  });

  it("offers a theme toggle", () => {
    expect(html).toContain("Switch to light theme");
  });
});
