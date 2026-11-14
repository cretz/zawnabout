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
  /** Gaps between frames over 25 ms, each a stutter. */
  long: number;
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
  // Only the dev server takes reports. The published page has nowhere to send them.
  panel.append(box);
  if (import.meta.env.DEV) panel.append(send);
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
  let [from, last, frames, worst, long] = [performance.now(), performance.now(), 0, 0, 0];
  return (now: number) => {
    if (wanted) {
      wanted = false;
      void post(grab());
    }
    frames++;
    worst = Math.max(worst, now - last);
    if (now - last > 25) long++;
    last = now;
    if (now - from < 2000) return;
    const [parts, slowest] = app.perf_report().split("\n");
    const fps = (frames * 1000) / (now - from);
    readings.push({
      at: new Date().toISOString(),
      fps: Math.round(fps * 10) / 10,
      worst: Math.round(worst),
      long,
      parts,
      slowest,
      view: JSON.parse(app.perf_view()),
    });
    if (readings.length > 10) readings.shift();
    const lines = parts.split("; ").slice(0, 10);
    box.textContent =
      `${fps.toFixed(1)} fps, worst gap ${worst.toFixed(0)} ms, ${long} over 25 ms, ` +
      `canvas ${canvas.width}x${canvas.height}\n` +
      lines.join("\n") +
      `\n${slowest.trim().split("; ").slice(0, 8).join("\n  ")}`;
    [from, frames, worst, long] = [now, 0, 0, 0];
  };
}

/** Pixels drawn per CSS pixel at most: the screen's own, up to 2. */
const sharpest = () => Math.min(window.devicePixelRatio || 1, 2);

function sizeCanvas(canvas: HTMLCanvasElement, scale: number) {
  const w = Math.max(1, Math.round(canvas.clientWidth * scale));
  const h = Math.max(1, Math.round(canvas.clientHeight * scale));
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
  }
}

/** Where Auto sharpness last settled on this device, to start there next time. */
const AUTO_KEY = "zawnabout.autoscale";
/** How far apart the sharpness levels Auto moves between are, in pixels per CSS pixel. */
const STEP = 0.125;

const percentile = (xs: number[], p: number) => [...xs].sort((a, b) => a - b)[Math.floor((xs.length - 1) * p)];

/**
 * Auto sharpness: how many pixels to draw for each CSS pixel, picked from how fast frames
 * come. It reads one second of frames at a time. Two slow seconds in a row draw fewer
 * pixels, as many fewer as the GPU's time says are needed. Three seconds with the GPU
 * well under its time draw a step or two more. After any change it waits a few seconds
 * before judging again, and it waits longer and longer before stepping back up where that
 * keeps turning out too much, so the picture settles instead of shifting about. Drawing
 * fewer pixels that does not help, such as when the browser holds frames back to save
 * power, is undone.
 */
function autoSharpness(log: boolean) {
  const clamp = (s: number) => Math.min(sharpest(), Math.max(1, Math.round(s / STEP) * STEP));
  let scale = clamp(Number(load(AUTO_KEY)) || sharpest());
  let gaps: number[] = [];
  let gpus: number[] = [];
  let [from, last, drew] = [performance.now(), 0, false];
  // Not judged again until then, after a change or while the game starts.
  let settle = performance.now() + 3000;
  let [slow, roomy] = [0, 0];
  // Stepping up waits this long after stepping down, doubling each time an up was undone.
  let [upWait, downAt, upAt] = [10_000, -Infinity, -Infinity];
  // A step down still being tried: where it came from, and how slow frames and the GPU
  // were before it.
  let tried: { scale: number; gap: number; busy: number | null } | null = null;
  // Stepping down did not help, so it waits until then.
  let noDownUntil = 0;
  const set = (s: number, why: string) => {
    if (log) console.log(`sharpness ${scale} to ${s}: ${why}`);
    scale = s;
    settle = performance.now() + 3000;
    [slow, roomy] = [0, 0];
    keep(AUTO_KEY, String(s));
  };
  return {
    scale: () => clamp(scale),
    /** One frame: when it began, and the GPU's time over a frame that finished, if one did. */
    tick(now: number, gpu: number) {
      // Only frames after drawn ones count, and gaps long enough to be a hidden tab say
      // nothing about drawing. A second of frames is judged at once, or more on a GPU
      // slow enough to give fewer than five in a second.
      if (drew && now - last < 2000) gaps.push(now - last);
      if (gpu >= 0) gpus.push(gpu);
      last = now;
      if (now - from < 1000 || gaps.length < 5) return;
      const [g, u] = [gaps, gpus];
      [gaps, gpus, from] = [[], [], now];
      if (now < settle) return;
      const gap = percentile(g, 0.75);
      // Under 50 frames a second, or too many stutters.
      const isSlow = gap > 20;
      const busy = u.length >= 5 ? percentile(u, 0.5) : null;
      if (tried) {
        // Waiting for the screen to refresh can hide a faster GPU in the gaps, so its
        // own time counts too.
        const quicker = busy !== null && tried.busy !== null && busy < tried.busy * 0.9;
        const helped = !isSlow || gap < tried.gap * 0.9 || quicker;
        if (!helped) {
          noDownUntil = now + 60_000;
          set(tried.scale, `fewer pixels did not help (${gap.toFixed(1)} ms frames)`);
        }
        tried = null;
        if (!helped) return;
      }
      slow = isSlow ? slow + 1 : 0;
      if (slow >= 2 && scale > 1 && now >= noDownUntil) {
        // Pixels cost GPU time roughly in proportion, so aim for 14 ms of it.
        const want = busy ? scale * Math.sqrt(14 / busy) : scale - STEP;
        const next = Math.max(1, scale - 4 * STEP, Math.min(scale - STEP, Math.floor(want / STEP) * STEP));
        if (now - upAt < 10_000) upWait = Math.min(upWait * 2, 300_000);
        tried = { scale, gap, busy };
        downAt = now;
        set(next, `slow (${gap.toFixed(1)} ms frames, GPU ${busy?.toFixed(1)} ms)`);
        return;
      }
      // A step up, or two, would still leave the GPU a fifth of a 60 Hz frame spare. Part
      // of the GPU's time does not grow with pixels, so this guesses high, and a step that
      // turns out too much comes back down.
      const fits = (s: number) => !isSlow && busy !== null && s <= sharpest() && busy * (s / scale) ** 2 < 13;
      roomy = fits(scale + STEP) ? roomy + 1 : 0;
      if (roomy >= 3 && now - downAt >= upWait) {
        upAt = now;
        set(fits(scale + 2 * STEP) ? scale + 2 * STEP : scale + STEP, `room to spare (GPU ${busy?.toFixed(1)} ms)`);
      }
    },
    drew(on: boolean) {
      drew = on;
    },
  };
}

/** The pixels to draw per CSS pixel for each Settings sharpness, Auto aside. */
const FIXED: Record<string, () => number> = {
  sharp: sharpest,
  balanced: () => Math.min(sharpest(), 1.5),
  fast: () => 1,
};

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
  const loading = $("loading");
  if (!("gpu" in navigator)) {
    loading.remove();
    $("nogpu").classList.remove("hidden");
    return;
  }
  await init();
  const auto = autoSharpness(perf);
  sizeCanvas(canvas, auto.scale());
  // Making the world holds the page for a few seconds, so let the loading dots show first.
  await new Promise((r) => requestAnimationFrame(() => setTimeout(r)));
  let app: App;
  try {
    app = await App.create(canvas, Number(query.get("seed") ?? 1));
  } catch (e) {
    console.error(e);
    loading.remove();
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
    const sharp = app.sharpness();
    if (sharp === "auto") auto.tick(now, app.take_gpu());
    const scale = FIXED[sharp]?.() ?? auto.scale();
    sizeCanvas(canvas, scale);
    app.set_ui_scale(sharpest() / scale);
    const host = app.frame(dt, canvas.width, canvas.height);
    auto.drew(app.drew());
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
    // The loading screen fades once the first frame is drawn.
    if (!loading.classList.contains("done")) {
      loading.classList.add("done");
      loading.addEventListener("transitionend", () => loading.remove());
    }
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

start();
