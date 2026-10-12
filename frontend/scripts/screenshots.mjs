// Playwright screenshot harness for the visual subagent.
//
// Boots a seeded session against a RUNNING stack and captures the key screens
// (login, rooms, chat with reactions/read receipts, dark + light themes) as
// PNGs under frontend/screenshots/ for review. This is a plain Node script —
// it is NOT part of the `make check` gate and CI never runs it.
//
// Requirements:
//   - `npm ci` (installs @playwright/test) and `npx playwright install chromium`
//   - Backend + frontend dev servers running (see README "Screenshots")
//
// Env: VITE_ORIGIN (default http://localhost:5173) — the Vite origin the
// browser visits; /api and /health are proxied to the backend there.
//      SCREENSHOTS_DIR (default frontend/screenshots)

import { mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

if (typeof WebSocket !== "function") {
  throw new Error(
    "this script needs the global WebSocket (Node 22+); run it with Node 22 or newer",
  );
}

const ORIGIN = process.env.VITE_ORIGIN ?? "http://localhost:5173";
const VIEWPORT = { width: 1280, height: 800 };
const OUT_DIR =
  process.env.SCREENSHOTS_DIR ??
  path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "screenshots");

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** The strong default password used by every seeded account. */
const PASSWORD = "correct horse battery staple";

function png(name) {
  return path.join(OUT_DIR, name);
}

// --- stack readiness -------------------------------------------------------

async function waitForStack() {
  const deadline = Date.now() + 20_000;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      const [origin, health] = await Promise.all([
        fetch(ORIGIN, { signal: AbortSignal.timeout(3_000) }),
        fetch(`${ORIGIN}/health`, { signal: AbortSignal.timeout(3_000) }),
      ]);
      if (origin.ok && health.ok) {
        return;
      }
      lastError = new Error(`frontend ${origin.status}, backend ${health.status}`);
    } catch (error) {
      lastError = error;
    }
    await sleep(500);
  }
  throw new Error(
    `the stack is not up at ${ORIGIN} (last error: ${lastError?.message})\n` +
      "Start it, then retry:\n" +
      "  backend : cd backend && cargo run        (set DATABASE_URL first)\n" +
      "  frontend: cd frontend && npm run dev\n" +
      "or run the whole stack with 'make dev-up' from the repo root.",
  );
}

// --- seeding via the public API + WebSocket ---------------------------------

async function api(pathname, { method = "GET", token = null, body = null } = {}) {
  const headers = {};
  if (body !== null) {
    headers["Content-Type"] = "application/json";
  }
  if (token !== null) {
    headers.Authorization = `Bearer ${token}`;
  }
  const response = await fetch(`${ORIGIN}/api${pathname}`, {
    method,
    headers,
    body: body === null ? undefined : JSON.stringify(body),
  });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`${method} /api${pathname} -> ${response.status}: ${text.slice(0, 200)}`);
  }
  if (response.status === 204) {
    return null;
  }
  const text = await response.text();
  return text.length === 0 ? null : JSON.parse(text);
}

async function register(handle) {
  const username = `${handle}_${Math.random().toString(36).slice(2, 8)}`;
  const registered = await api("/auth/register", {
    method: "POST",
    body: { username, password: PASSWORD },
  });
  return { token: registered.token, id: registered.user.id, username, password: PASSWORD };
}

/**
 * Open a WS session as `token` in `roomId`, send `frames`, and resolve with
 * the collected server events once every `send` frame has been echoed back.
 */
function sendFrames(token, roomId, frames) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(
      `${ORIGIN.replace(/^http/, "ws")}/api/ws?token=${encodeURIComponent(token)}&room_id=${encodeURIComponent(roomId)}`,
    );
    const expectedSends = frames.filter((frame) => frame.type === "send").length;
    const events = [];
    let timer = setTimeout(() => {
      socket.close();
      reject(new Error("timed out waiting for message echoes on the seed socket"));
    }, 10_000);

    socket.addEventListener("open", () => {
      for (const frame of frames) {
        socket.send(JSON.stringify(frame));
      }
    });
    socket.addEventListener("message", (event) => {
      let data;
      try {
        data = JSON.parse(event.data);
      } catch {
        clearTimeout(timer);
        socket.close();
        reject(new Error("seed socket sent non-JSON data"));
        return;
      }
      events.push(data);
      if (events.filter((item) => item.type === "message").length >= expectedSends) {
        clearTimeout(timer);
        socket.close();
        resolve(events);
      }
    });
    socket.addEventListener("close", () => clearTimeout(timer));
    socket.addEventListener("error", () => {
      clearTimeout(timer);
      socket.close();
      reject(new Error("seed WebSocket errored"));
    });
  });
}

// --- browser helpers ---------------------------------------------------------

async function login(page, user) {
  await page.locator('input[autocomplete="username"]').fill(user.username);
  await page.locator('input[autocomplete="current-password"]').fill(user.password);
  await page.locator('button[type="submit"]').click();
  await page.locator(".rooms").waitFor();
}

async function newContext(browser, theme) {
  const context = await browser.newContext({
    viewport: VIEWPORT,
    colorScheme: theme,
    permissions: [{ origin: ORIGIN, permission: "notifications" }],
  });
  if (theme === "light") {
    await context.addInitScript(() => {
      localStorage.setItem("livechat.theme", "light");
    });
  }
  return context;
}

// --- captures ----------------------------------------------------------------

async function captureAuth(page, name) {
  await page.goto(ORIGIN, { waitUntil: "domcontentloaded" });
  await page.locator(".auth").waitFor();
  await page.screenshot({ path: png(name) });
}

async function captureRooms(page, roomName, name) {
  const row = page.locator(".rooms__list button", { hasText: roomName });
  await row.first().waitFor();
  await row.first().scrollIntoViewIfNeeded();
  await sleep(150);
  await page.screenshot({ path: png(name) });
}

async function captureChat(page, roomName, name) {
  const row = page.locator(".rooms__list button", { hasText: roomName });
  await row.first().waitFor();
  await row.first().click();
  await page.locator(".message").first().waitFor();
  await page.locator(".status-open").waitFor();
  await sleep(300);
  await page.screenshot({ path: png(name) });
}

// --- main --------------------------------------------------------------------

async function main() {
  await waitForStack();
  await mkdir(OUT_DIR, { recursive: true });

  // Seed a two-person room with messages, reactions, and a read receipt so the
  // chat captures show the realtime features, not the empty state.
  const alice = await register("alice");
  const bob = await register("bob");
  const roomName = `lobby-${Math.random().toString(36).slice(2, 6)}`;
  const room = await api("/rooms", {
    method: "POST",
    token: alice.token,
    body: { name: roomName },
  });
  await api(`/rooms/${room.id}/join`, { method: "POST", token: bob.token });

  const aliceEvents = await sendFrames(alice.token, room.id, [
    { type: "send", body: "Hey everyone — first pass on the new shell." },
    { type: "send", body: "Docs live at https://example.com — links render inline." },
    { type: "send", body: "Give this one a reaction 👋" },
  ]);
  const aliceMessageIds = aliceEvents
    .filter((event) => event.type === "message")
    .map((event) => event.message.id);
  const [, m2, m3] = aliceMessageIds;

  const bobEvents = await sendFrames(bob.token, room.id, [
    { type: "react", message_id: m2, emoji: "🎉" },
    { type: "react", message_id: m3, emoji: "👍" },
    { type: "send", body: "Looking sharp — teal reads well on the dark." },
  ]);
  const bobMsgId = bobEvents.find((event) => event.type === "message").message.id;

  let browser;
  try {
    browser = await chromium.launch();
  } catch (error) {
    throw new Error(
      `could not launch Chromium: ${error.message.split("\n")[0]}\n` +
        "On Linux the Playwright browser usually needs system libraries before it runs:\n" +
        "  npx playwright install --with-deps chromium\n" +
        'then start the stack (see README "Screenshots") and retry.',
    );
  }

  try {
    // Dark theme: auth, room list with unread badge, then the live chat.  Bob
    // marks the room read while Alice is watching so "Seen by bob" appears.
    const dark = await newContext(browser, "dark");
    const darkPage = await dark.newPage();
    await captureAuth(darkPage, "01-auth-dark.png");
    await login(darkPage, alice);
    await captureRooms(darkPage, roomName, "03-rooms-dark.png");
    await captureChat(darkPage, roomName, "04-chat-dark.png");
    await api(`/rooms/${room.id}/read`, {
      method: "POST",
      token: bob.token,
      body: { message_id: bobMsgId },
    });
    await darkPage.locator(".message__seen", { hasText: "Seen by bob" }).waitFor();
    await sleep(250);
    await darkPage.screenshot({ path: png("05-chat-seen-dark.png") });
    await dark.close();

    // Light theme: auth + the same chat.
    const light = await newContext(browser, "light");
    const lightPage = await light.newPage();
    await captureAuth(lightPage, "02-auth-light.png");
    await login(lightPage, alice);
    await captureChat(lightPage, roomName, "06-chat-light.png");
    await light.close();

    console.log(`Saved screenshots to ${OUT_DIR}`);
  } finally {
    await browser.close();
  }
}

main().catch((error) => {
  console.error(`\nscreenshots failed: ${error.message}`);
  process.exit(1);
});
