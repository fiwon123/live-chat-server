// Playwright screenshot harness for the README.
//
// Boots a seeded session against a RUNNING stack and captures one PNG per key
// feature (an overview/hero, auth, room list, live chat with a read receipt,
// the hover reaction bar, the composer emoji picker, the typing indicator, and
// the About page) under frontend/screenshots/ for review. All captures are in
// the dark theme — the README ships one consistent set. This is a plain Node
// script — it is NOT part of the `make check` gate and CI never runs it.
//
// Requirements:
//   - `npm ci` (installs @playwright/test) and `npx playwright install chromium`
//   - Backend + frontend dev servers running (see README "Regenerating the
//     screenshots")
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
    const error = new Error(
      `${method} /api${pathname} -> ${response.status}: ${text.slice(0, 200)}`,
    );
    error.status = response.status;
    throw error;
  }
  if (response.status === 204) {
    return null;
  }
  const text = await response.text();
  return text.length === 0 ? null : JSON.parse(text);
}

/**
 * Register `handle`, or log it back in if it already exists. Reusing the same
 * two accounts across runs keeps the author names clean in every capture.
 */
async function ensureUser(handle) {
  try {
    const registered = await api("/auth/register", {
      method: "POST",
      body: { username: handle, password: PASSWORD },
    });
    return { token: registered.token, id: registered.user.id, username: handle };
  } catch (error) {
    if (error.status !== 409) {
      throw error;
    }
    const loggedIn = await api("/auth/login", {
      method: "POST",
      body: { username: handle, password: PASSWORD },
    });
    return { token: loggedIn.token, id: loggedIn.user.id, username: handle };
  }
}

/**
 * Delete every room `userId` owns. The backend caps owned rooms per user
 * (`MAX_ROOMS_PER_USER`, default 5), so re-running this harness with the same
 * account would eventually hit the cap; clearing them first keeps runs
 * repeatable and the room list tidy.
 */
async function deleteOwnedRooms(token, userId) {
  const rooms = await api("/rooms", { token });
  for (const room of rooms) {
    if (room.created_by === userId) {
      await api(`/rooms/${room.id}`, { method: "DELETE", token });
    }
  }
}

/** Open a WS session as `token` in `roomId`, resolved once the socket opens. */
function openSocket(token, roomId) {
  const url = `${ORIGIN.replace(/^http/, "ws")}/api/ws?token=${encodeURIComponent(token)}&room_id=${encodeURIComponent(roomId)}`;
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(url);
    const events = [];
    const waiters = [];
    // Events are consumed in order: `waitFor` scans from `cursor` so successive
    // calls (one per seeded send) resolve to distinct echoes, not the same one.
    let cursor = 0;
    const take = (predicate) => {
      for (let i = cursor; i < events.length; i += 1) {
        if (predicate(events[i])) {
          cursor = i + 1;
          return events[i];
        }
      }
      return undefined;
    };
    socket.addEventListener("open", () =>
      resolve({
        send: (frame) => socket.send(JSON.stringify(frame)),
        close: () => socket.close(),
        /** Resolve with the next (unconsumed) event matching `predicate`. */
        waitFor(predicate, label = "event", timeoutMs = 8_000) {
          const found = take(predicate);
          if (found) {
            return Promise.resolve(found);
          }
          return new Promise((res, rej) => {
            const timer = setTimeout(
              () => rej(new Error(`timed out waiting for ${label}`)),
              timeoutMs,
            );
            waiters.push({ predicate, resolve: res, timer });
          });
        },
      }),
    );
    socket.addEventListener("message", (event) => {
      let data;
      try {
        data = JSON.parse(event.data);
      } catch {
        return;
      }
      events.push(data);
      for (let i = waiters.length - 1; i >= 0; i -= 1) {
        if (waiters[i].predicate(data)) {
          const [waiter] = waiters.splice(i, 1);
          clearTimeout(waiter.timer);
          cursor = events.length;
          waiter.resolve(data);
        }
      }
    });
    socket.addEventListener("error", () => reject(new Error("seed WebSocket errored")));
  });
}

// --- browser helpers ---------------------------------------------------------

async function login(page, user) {
  await page.locator('input[autocomplete="username"]').fill(user.username);
  await page.locator('input[autocomplete="current-password"]').fill(PASSWORD);
  await page.locator('button[type="submit"]').click();
  await page.locator(".rooms").waitFor();
}

async function newContext(browser) {
  const context = await browser.newContext({
    viewport: VIEWPORT,
    colorScheme: "dark",
  });
  // Scope the granted permission to the app origin (the API is proxied there).
  await context.grantPermissions(["notifications"], { origin: ORIGIN });
  return context;
}

// --- captures ----------------------------------------------------------------

async function captureAuth(page, name) {
  await page.goto(ORIGIN, { waitUntil: "domcontentloaded" });
  await page.locator(".auth").waitFor();
  await page.screenshot({ path: png(name) });
}

async function openRoom(page, roomName) {
  const row = page.locator(".rooms__list button", { hasText: roomName });
  await row.first().waitFor();
  await row.first().scrollIntoViewIfNeeded();
  return row.first();
}

async function captureRooms(page, roomName, name) {
  await openRoom(page, roomName);
  await sleep(150);
  await page.screenshot({ path: png(name) });
}

async function enterRoom(page, roomName) {
  const row = await openRoom(page, roomName);
  await row.click();
  await page.locator(".message").first().waitFor();
  await page.locator(".status-open").waitFor();
}

/**
 * Trigger a fresh ephemeral read receipt for the connected viewer and capture
 * the chat once it lands. Read receipts broadcast only on a real cursor
 * advance, so each capture passes a cursor newer than the last one. The caller
 * must already be in the room (see `enterRoom`).
 */
async function captureReceipt(page, { roomId, token, messageId, name }) {
  await api(`/rooms/${roomId}/read`, { method: "POST", token, body: { message_id: messageId } });
  await page.locator(".message__seen", { hasText: "Seen by bob" }).waitFor();
  await sleep(300);
  await page.screenshot({ path: png(name) });
}

/**
 * Hover an incoming message so its reaction bar is revealed. Own messages have
 * no bar (by design), so the target is the last `.message:not(.own)`.
 */
async function captureReactionBar(page, name) {
  const target = page.locator(".message:not(.own)").last();
  await target.scrollIntoViewIfNeeded();
  await target.hover();
  await sleep(250);
  await page.screenshot({ path: png(name) });
}

async function captureEmojiPicker(page, name) {
  await page.locator(".composer__emoji").click();
  await page.locator(".emoji__pop").waitFor();
  await sleep(150);
  await page.screenshot({ path: png(name) });
}

async function captureTyping(page, name) {
  await page.locator(".typing__dots").waitFor();
  await sleep(150);
  await page.screenshot({ path: png(name) });
}

async function captureAbout(page, name) {
  await page.goto(`${ORIGIN}/#/about`, { waitUntil: "domcontentloaded" });
  await page.locator(".about").waitFor();
  await sleep(200);
  await page.screenshot({ path: png(name) });
}

// --- main --------------------------------------------------------------------

async function main() {
  await waitForStack();
  await mkdir(OUT_DIR, { recursive: true });

  // Seed a two-person room with messages, reactions, and a read receipt so the
  // chat captures show the realtime features, not the empty state.
  const alice = await ensureUser("alice");
  const bob = await ensureUser("bob");
  await deleteOwnedRooms(alice.token, alice.id);

  const roomName = `lobby-${Math.random().toString(36).slice(2, 6)}`;
  const room = await api("/rooms", {
    method: "POST",
    token: alice.token,
    body: { name: roomName },
  });
  await api(`/rooms/${room.id}/join`, { method: "POST", token: bob.token });

  // Alice posts first (her own messages render right-aligned, without a bar).
  const aliceSocket = await openSocket(alice.token, room.id);
  const aliceBodies = [
    "Hey everyone — first pass on the new shell.",
    "Docs live at https://example.com — links render inline.",
    "Give this one a reaction 👋",
  ];
  const aliceMessageIds = [];
  for (const body of aliceBodies) {
    aliceSocket.send({ type: "send", body });
    const event = await aliceSocket.waitFor(
      (item) => item.type === "message" && item.message.username === "alice",
      `alice echo of "${body}"`,
    );
    aliceMessageIds.push(event.message.id);
  }
  aliceSocket.close();
  const [, m2, m3] = aliceMessageIds;

  // Bob stays connected: his presence keeps the "online" cluster populated, and
  // his socket later emits the typing frame. He reacts, then posts two messages
  // (left-aligned to Alice, so they carry the hover reaction bar).
  const bobSocket = await openSocket(bob.token, room.id);
  bobSocket.send({ type: "react", message_id: m2, emoji: "🎉" });
  bobSocket.send({ type: "react", message_id: m3, emoji: "👍" });
  for (const body of [
    "Looking sharp — teal reads well on the dark.",
    "Ship it — history survives a reload.",
  ]) {
    bobSocket.send({ type: "send", body });
    await bobSocket.waitFor(
      (item) => item.type === "message" && item.message.username === "bob",
      `bob echo of "${body}"`,
    );
  }
  let browser;
  try {
    browser = await chromium.launch();
  } catch (error) {
    throw new Error(
      `could not launch Chromium: ${error.message.split("\n")[0]}\n` +
        "On Linux the Playwright browser usually needs system libraries before it runs:\n" +
        "  npx playwright install --with-deps chromium\n" +
        'then start the stack (see README "Regenerating the screenshots") and retry.',
    );
  }

  try {
    // --- dark theme: the hero, then one capture per feature ------------------
    const context = await newContext(browser);
    const page = await context.newPage();
    await captureAuth(page, "01-auth.png");
    await login(page, alice);
    await captureRooms(page, roomName, "02-rooms.png");

    // Hero: the app in use (sidebar + conversation), before any ephemeral
    // overlay such as the read receipt is triggered.
    await enterRoom(page, roomName);
    await sleep(600);
    await page.screenshot({ path: png("00-overview.png") });

    // Ephemeral overlays first, while the room is otherwise clean: hover
    // reactions, then the composer emoji picker, then a typing indicator. The
    // read receipt is left until last so it does not sit under every capture.
    await captureReactionBar(page, "04-reactions.png");
    await captureEmojiPicker(page, "05-emoji-picker.png");
    // Close the picker so it does not bleed into the typing capture.
    await page.keyboard.press("Escape");
    bobSocket.send({ type: "typing" });
    await captureTyping(page, "06-typing.png");
    // Let the typing indicator expire (TTL ~3.5s) so it does not ride into the
    // read-receipt capture.
    await sleep(3_700);

    // Live chat: advance Bob's cursor to Alice's last message so the
    // "Seen by bob" receipt rides in on a live broadcast.
    await captureReceipt(page, {
      roomId: room.id,
      token: bob.token,
      messageId: m3,
      name: "03-chat.png",
    });
    await captureAbout(page, "07-about.png");
    await context.close();

    bobSocket.close();
    console.log(`Saved screenshots to ${OUT_DIR}`);
  } finally {
    bobSocket.close();
    await browser.close();
  }
}

main().catch((error) => {
  console.error(`\nscreenshots failed: ${error.message}`);
  process.exit(1);
});
