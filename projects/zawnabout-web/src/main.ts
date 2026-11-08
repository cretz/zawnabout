import init, { App } from "../pkg/zawnabout_web.js";
import { setupDebug } from "./debug.ts";
import { setupMinimap } from "./minimap.ts";
import { setupNotes } from "./notes.ts";
import { setupSound } from "./sound.ts";
import { setupTouch } from "./touch.ts";

const query = new URLSearchParams(location.search);
const debug = query.has("debug");
// `?perf` shows how long frames take on this device, and what in them takes the time.
const perf = query.has("perf");
const SETTINGS_KEY = "zawnabout.settings";
// What the game keeps in the browser: settings, keys, and the save slot.
const PREFS_KEY = "zawnabout.prefs";
const KEYS_KEY = "zawnabout.keys";
const SAVE_KEY = "zawnabout.save.a";

// Keys the game uses that browsers would otherwise scroll or act on.
const GAME_KEYS = new Set(["Space", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "ShiftLeft", "ShiftRight", "Tab"]);

declare global {
  interface Window {
    zawnabout?: {
      app: App;
      cmd: (line: string) => string;
      run: (script: string) => void;
      shots: string[];
      logs: string[];
      ready: boolean;
    };
  }
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function load(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function keep(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Kept for this visit only.
  }
}

/** One two-second reading of the perf readout. */
interface Reading {
  at: string;
  fps: number;
  worst: number;
  parts: string;
  slowest: string;
  view: unknown;
}

/**
 * A readout of frame times, refreshed every two seconds, with a button that sends the
 * last twenty seconds of it to the dev server for the agent to read. Returns the
 * per-frame tick.
 */
function setupPerf(app: App, canvas: HTMLCanvasElement) {
  app.perf_start();
  const panel = document.createElement("div");
  panel.id = "perf";
  const box = document.createElement("pre");
  const send = document.createElement("button");
  send.textContent = "Send perf";
  panel.append(box, send);
  document.body.append(panel);
  const readings: Reading[] = [];
  for (const ev of ["pointerdown", "pointerup", "touchstart", "touchend", "mousedown", "mouseup"]) {
    send.addEventListener(ev, (e) => e.stopPropagation());
  }
  // The picture is read right after the next frame is drawn, while it is still there.
  let wanted = false;
  send.addEventListener("click", (e) => {
    e.stopPropagation();
    send.textContent = "Sending...";
    app.redraw();
    wanted = true;
  });
  const post = async (shot: string) => {
    const report = {
      sent: new Date().toISOString(),
      agent: navigator.userAgent,
      dpr: window.devicePixelRatio,
      screen: [screen.width, screen.height],
      canvas: [canvas.width, canvas.height],
      readings,
      shot,
    };
    try {
      const res = await fetch("/__perf", { method: "POST", body: JSON.stringify(report) });
      send.textContent = res.ok ? "Sent, thanks!" : `Failed (${res.status})`;
    } catch {
      send.textContent = "No server to send to";
    }
    setTimeout(() => (send.textContent = "Send perf"), 3000);
  };
  const grab = () => {
    const scale = Math.min(1, 1280 / canvas.width);
    const c = document.createElement("canvas");
    c.width = Math.round(canvas.width * scale);
    c.height = Math.round(canvas.height * scale);
    c.getContext("2d")?.drawImage(canvas, 0, 0, c.width, c.height);
    return c.toDataURL("image/jpeg", 0.8);
  };
  let [from, last, frames, worst] = [performance.now(), performance.now(), 0, 0];
  return (now: number) => {
    if (wanted) {
      wanted = false;
      void post(grab());
    }
    frames++;
    worst = Math.max(worst, now - last);
    last = now;
    if (now - from < 2000) return;
    const [parts, slowest] = app.perf_report().split("\n");
    const fps = (frames * 1000) / (now - from);
    readings.push({
      at: new Date().toISOString(),
      fps: Math.round(fps * 10) / 10,
      worst: Math.round(worst),
      parts,
      slowest,
      view: JSON.parse(app.perf_view()),
    });
    if (readings.length > 10) readings.shift();
    const lines = parts.split("; ").slice(0, 10);
    box.textContent =
      `${fps.toFixed(1)} fps, worst gap ${worst.toFixed(0)} ms, ` +
      `canvas ${canvas.width}x${canvas.height}\n` +
      lines.join("\n") +
      `\n${slowest.trim().split("; ").slice(0, 8).join("\n  ")}`;
    [from, frames, worst] = [now, 0, 0];
  };
}

function sizeCanvas(canvas: HTMLCanvasElement) {
  // Cap the pixel ratio so tablets and phones stay smooth. Touch screens get a bit less.
  const touch = window.matchMedia("(pointer: coarse)").matches;
  const dpr = Math.min(window.devicePixelRatio || 1, touch ? 1.5 : 2);
  const w = Math.max(1, Math.round(canvas.clientWidth * dpr));
  const h = Math.max(1, Math.round(canvas.clientHeight * dpr));
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
  }
}

function pollGamepad(app: App, state: { had: boolean }) {
  const pad = navigator.getGamepads?.().find((p) => p && p.connected);
  if (pad) {
    app.gamepad(Float32Array.from(pad.axes), Float32Array.from(pad.buttons, (b) => b.value));
    state.had = true;
  } else if (state.had) {
    app.gamepad(new Float32Array(), new Float32Array());
    state.had = false;
  }
}

/** Offers the save as a file to keep. */
function download(text: string) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
  a.download = "zawnabout-save.txt";
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

/** Asks for a save file and loads it. */
function pickSave(app: App) {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".txt,text/plain";
  input.addEventListener("change", async () => {
    const file = input.files?.[0];
    if (!file) return;
    try {
      app.load_save(await file.text());
      const text = app.save_text();
      if (text) keep(SAVE_KEY, text);
    } catch (e) {
      alert(`That file is not a zawnabout save (${e}).`);
    }
  });
  input.click();
}

async function start() {
  const canvas = $<HTMLCanvasElement>("game");
  if (!("gpu" in navigator)) {
    $("nogpu").classList.remove("hidden");
    return;
  }
  await init();
  sizeCanvas(canvas);
  let app: App;
  try {
    app = await App.create(canvas, Number(query.get("seed") ?? 1));
  } catch (e) {
    console.error(e);
    $("nogpu").classList.remove("hidden");
    return;
  }

  const saved = load(SETTINGS_KEY);
  if (saved) {
    for (const line of saved.split("\n")) {
      if (line.trim()) app.command(line);
    }
  }
  const saveSettings = () => keep(SETTINGS_KEY, app.dump());
  if (query.get("play") === "0") app.command("play off");

  // Settings, keys and the saved game, as they were left. Settings from before the
  // journal kept sound and pausing under keys of their own.
  let prefs = load(PREFS_KEY);
  if (prefs === null) {
    prefs = `sound ${load("zawnabout.sound") === "0" ? "off" : "on"}\npauseaway ${load("zawnabout.pauseaway") === "0" ? "off" : "on"}\n`;
  }
  app.set_prefs_text(prefs);
  const keys = load(KEYS_KEY);
  if (keys) app.set_bindings_text(keys);
  const game = load(SAVE_KEY);
  if (game) {
    try {
      app.load_save(game);
    } catch (e) {
      console.warn("save", e);
    }
  }

  const touch = query.has("touch") || matchMedia("(pointer: coarse)").matches;
  app.set_touch(touch);
  // Full screen where the browser allows it. iPhone Safari does not, so there the button
  // explains Add to Home Screen, which opens without any browser bars.
  const full = $("full-btn");
  if (touch || document.fullscreenEnabled) full.classList.remove("hidden");
  full.addEventListener("click", async () => {
    if (document.fullscreenElement) {
      await document.exitFullscreen();
    } else if (document.fullscreenEnabled) {
      await document.documentElement.requestFullscreen({ navigationUI: "hide" }).catch(() => {});
    } else {
      alert("For full screen on iPhone: tap Share, then Add to Home Screen, and open the dragon from there.");
    }
  });

  const notes = setupNotes(app, canvas, touch);
  setupSound(app);
  // The atlas pauses the world while it is open.
  const minimap = setupMinimap(app, touch, (on) => app.command(on ? "pause" : "resume"));
  // A tap on the touch stick area works the journal first, then picks a note spot in
  // notes mode, else a landing spot.
  if (touch)
    setupTouch(app, (x, y) => {
      if (notes.active()) {
        if (!notes.writing()) notes.pick(x, y);
      } else if (!app.ui_click(x / innerWidth, y / innerHeight)) {
        app.tap(x / innerWidth, y / innerHeight);
      }
    });

  $("book-btn").addEventListener("click", () => app.open_book());

  // The game starts on its journal, open at help, paused.
  if (!query.has("nowelcome")) app.open_book();

  // Leaving the tab or the window opens the journal too, unless that is turned off.
  const leave = () => {
    if (app.pause_away() && !notes.active() && !query.has("nowelcome")) app.open_book();
  };
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) leave();
  });
  window.addEventListener("blur", leave);

  const typing = (e: Event) => e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
  window.addEventListener("keydown", (e) => {
    if (typing(e)) return;
    if (e.code === "Backquote" && debugPanel) {
      debugPanel.toggle();
      return;
    }
    if ((e.code === "KeyN" && !app.halted()) || (e.code === "Escape" && notes.active())) {
      notes.toggle();
      return;
    }
    // The journal and cards take keys first. Held keys only repeat for flying.
    if (!notes.active() && !e.repeat && app.ui_key(e.code)) {
      e.preventDefault();
      return;
    }
    if (GAME_KEYS.has(e.code)) e.preventDefault();
    app.key(e.code, true, e.repeat);
  });
  window.addEventListener("keyup", (e) => {
    if (typing(e)) return;
    app.key(e.code, false, false);
  });
  window.addEventListener("blur", () => app.blur());
  // Click or tap the world to fly there and land. Drag it, or hold the right button, to
  // look around. The journal and cards take clicks first.
  let press: { x: number; y: number; at: number; id: number; drag: boolean } | null = null;
  canvas.addEventListener("pointerdown", (e) => {
    // The right button, or Ctrl with a click on a Mac, looks round from the first move.
    const look = e.pointerType === "mouse" && (e.button === 2 || (e.button === 0 && e.ctrlKey));
    press = { x: e.clientX, y: e.clientY, at: performance.now(), id: e.pointerId, drag: look };
    if (look) canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    app.ui_hover(e.clientX / canvas.clientWidth, e.clientY / canvas.clientHeight);
    if (!press || e.pointerId !== press.id || notes.active() || app.halted()) return;
    if (!press.drag && Math.hypot(e.clientX - press.x, e.clientY - press.y) >= 12) {
      press.drag = true;
      canvas.setPointerCapture(e.pointerId);
    }
    if (press.drag) app.look_drag(e.movementX || 0, e.movementY || 0);
  });
  canvas.addEventListener("pointerup", (e) => {
    const p = press;
    press = null;
    if (!p || notes.active() || p.drag) return;
    if (Math.hypot(e.clientX - p.x, e.clientY - p.y) < 12 && performance.now() - p.at < 400) {
      const nx = e.clientX / canvas.clientWidth;
      const ny = e.clientY / canvas.clientHeight;
      if (!app.ui_click(nx, ny)) app.tap(nx, ny);
    }
  });
  canvas.addEventListener("pointercancel", () => (press = null));
  window.addEventListener("contextmenu", (e) => e.preventDefault());

  const debugPanel = debug ? setupDebug(app, saveSettings) : null;

  const api = {
    app,
    cmd: (line: string) => app.command(line),
    run: (script: string) => app.run_script(script),
    shots: [] as string[],
    logs: [] as string[],
    ready: true,
  };
  window.zawnabout = api;

  // What the game asks of the page: keep its save, settings and keys, or work files.
  const answer = () => {
    for (const ask of app.take_asks().split("\n")) {
      if (ask === "save") {
        const text = app.save_text();
        if (text) keep(SAVE_KEY, text);
      } else if (ask === "prefs") keep(PREFS_KEY, app.prefs_text());
      else if (ask === "keys") keep(KEYS_KEY, app.bindings_text());
      else if (ask === "export") {
        const text = app.save_text();
        if (text) download(text);
      } else if (ask === "import") pickSave(app);
      else if (ask === "source") window.open("https://github.com/cretz/zawnabout", "_blank", "noopener");
    }
  };

  // A postcard's picture: the frame just drawn, shrunk on a page canvas and handed back.
  const shrink = document.createElement("canvas");
  shrink.width = 320;
  shrink.height = 180;
  const snap = () => {
    const ctx = shrink.getContext("2d", { willReadFrequently: true });
    if (!ctx) return;
    const s = Math.min(canvas.width / 16, canvas.height / 9);
    const [w, h] = [s * 16, s * 9];
    ctx.drawImage(canvas, (canvas.width - w) / 2, (canvas.height - h) / 2, w, h, 0, 0, 320, 180);
    const data = ctx.getImageData(0, 0, 320, 180).data;
    app.take_snap(new Uint8Array(data.buffer), 320, 180);
  };

  // Leaving the page, or hiding it, keeps the game where it is, to carry on next time.
  const keepGame = () => {
    const text = app.save_text();
    if (text) keep(SAVE_KEY, text);
  };
  addEventListener("pagehide", keepGame);
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") keepGame();
  });

  const perfShow = perf ? setupPerf(app, canvas) : null;
  const pad = { had: false };
  let last = performance.now();
  const loop = (now: number) => {
    const dt = Math.min((now - last) / 1000, 0.25);
    last = now;
    pollGamepad(app, pad);
    sizeCanvas(canvas);
    const host = app.frame(dt, canvas.width, canvas.height);
    answer();
    // The page's own buttons and touch controls step aside while the journal is up. The
    // big map pauses the game too, but it is page chrome itself, so it stays.
    document.body.classList.toggle("halted", app.halted() && !notes.active() && !minimap.atlas());
    const logs = app.take_logs();
    if (logs) {
      api.logs.push(...logs.split("\n"));
      if (debug) console.log(logs);
    }
    for (const line of host ? host.split("\n") : []) {
      const [cmd] = line.split(" ");
      if (cmd === "shot") api.shots.push(canvas.toDataURL("image/png"));
      if (cmd === "save-trace") api.logs.push(app.take_trace() ?? "");
      if (cmd === "snap") snap();
    }
    debugPanel?.update();
    perfShow?.(now);
    minimap.update(notes.active() ? notes.spots() : []);
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

start();
