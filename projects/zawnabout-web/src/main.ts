import init, { App } from "../pkg/zawnabout_web.js";
import { setupDebug } from "./debug.ts";
import { setupMinimap } from "./minimap.ts";
import { setupNotes } from "./notes.ts";
import { setupTouch } from "./touch.ts";

const query = new URLSearchParams(location.search);
const debug = query.has("debug");
const SETTINGS_KEY = "zawnabout.settings";

// Keys the game uses that browsers would otherwise scroll or act on.
const GAME_KEYS = new Set(["Space", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "ShiftLeft", "ShiftRight"]);

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

function controlsHtml(touch: boolean): string {
  const rows = touch
    ? [
        ["Left side", "Drag to steer"],
        ["Flap", "Tap, or hold to keep flapping"],
        ["Dive", "Hold to tuck your wings"],
        ["Brake", "Hold to slow down, land or turn tight"],
        ["Brake + Flap", "Hover in place"],
        ["Fire", "Hold to breathe fire. Light the beacons!"],
        ["On the ground", "Push the stick all the way to run"],
        ["Tap a glowing spot", "Fly there and land"],
        ["Pencil button", "Notes mode: pause, look around, leave notes"],
      ]
    : [
        ["<kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd> or arrows", "Steer, walk"],
        ["<kbd>Space</kbd>", "Flap (tap faster to flap harder)"],
        ["<kbd>Shift</kbd>", "Tuck wings and dive"],
        ["<kbd>E</kbd>, <kbd>Q</kbd> or <kbd>X</kbd>", "Brake: slow down, land, turn tight"],
        ["Brake + Flap", "Hover in place"],
        ["<kbd>F</kbd> or <kbd>R</kbd>", "Breathe fire. Light the beacons!"],
        ["Hold <kbd>W</kbd> on the ground", "Run"],
        ["Click a glowing spot", "Fly there and land"],
        ["<kbd>N</kbd>", "Notes mode: pause, fly around, leave notes"],
        ["<kbd>Esc</kbd> or <kbd>P</kbd>", "Pause"],
        ["Gamepad", "Stick, A flap, X fire, B or Y brake, triggers"],
      ];
  return rows.map(([k, v]) => `<span class="keys">${k}</span><span>${v}</span>`).join("");
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

  const saved = localStorage.getItem(SETTINGS_KEY);
  if (saved) {
    for (const line of saved.split("\n")) {
      if (line.trim()) app.command(line);
    }
  }
  const saveSettings = () => localStorage.setItem(SETTINGS_KEY, app.dump());

  const touch = query.has("touch") || matchMedia("(pointer: coarse)").matches;
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
  const minimap = setupMinimap(app, touch);
  // A tap on the touch stick area picks a note spot in notes mode, else a landing spot.
  if (touch)
    setupTouch(app, (x, y) => {
      if (!notes.active()) app.tap(x / innerWidth, y / innerHeight);
      else if (!notes.writing()) notes.pick(x, y);
    });

  const welcome = $("welcome");
  $("controls").innerHTML = controlsHtml(touch);
  if (touch) $("start-hint").textContent = "Tap to start";
  // The welcome card doubles as the pause screen: the world waits while it is up.
  let started = false;
  const showHelp = (show: boolean) => {
    if (show === !welcome.classList.contains("hidden")) return;
    welcome.classList.toggle("hidden", !show);
    if (show && started) {
      $("welcome-title").textContent = "Paused";
      $("start-hint").textContent = touch ? "Tap to keep flying" : "Press Esc or any key to keep flying";
    }
    if (!show) {
      started = true;
      app.blur();
    }
    app.command(show ? "pause" : "resume");
  };
  if (!query.has("nowelcome")) showHelp(true);
  else started = true;
  // Leaving the tab pauses too.
  document.addEventListener("visibilitychange", () => {
    if (document.hidden && !notes.active()) showHelp(true);
  });
  $("help-btn").addEventListener("click", () => showHelp(welcome.classList.contains("hidden")));
  welcome.addEventListener("pointerdown", () => showHelp(false));

  const typing = (e: Event) => e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
  window.addEventListener("keydown", (e) => {
    if (typing(e)) return;
    if (e.code === "Backquote" && debugPanel) {
      debugPanel.toggle();
      return;
    }
    if (e.code === "KeyN" || (e.code === "Escape" && notes.active())) {
      showHelp(false);
      notes.toggle();
      return;
    }
    if (e.code === "KeyH" || e.code === "Escape" || e.code === "KeyP") {
      showHelp(welcome.classList.contains("hidden"));
      return;
    }
    if (!welcome.classList.contains("hidden")) {
      // The key that closes the card only closes it.
      showHelp(false);
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
  // Click or tap the world to fly there and land. Drag it to look around.
  let press: { x: number; y: number; at: number; id: number; drag: boolean } | null = null;
  canvas.addEventListener("pointerdown", (e) => {
    press = { x: e.clientX, y: e.clientY, at: performance.now(), id: e.pointerId, drag: false };
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!press || e.pointerId !== press.id || notes.active()) return;
    if (!press.drag && Math.hypot(e.clientX - press.x, e.clientY - press.y) >= 12) {
      press.drag = true;
      canvas.setPointerCapture(e.pointerId);
    }
    if (press.drag) app.look_drag(e.movementX || 0, e.movementY || 0);
  });
  canvas.addEventListener("pointerup", (e) => {
    showHelp(false);
    const p = press;
    press = null;
    if (!p || notes.active() || p.drag) return;
    if (Math.hypot(e.clientX - p.x, e.clientY - p.y) < 12 && performance.now() - p.at < 400) {
      app.tap(e.clientX / canvas.clientWidth, e.clientY / canvas.clientHeight);
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

  const pad = { had: false };
  let last = performance.now();
  const loop = (now: number) => {
    const dt = Math.min((now - last) / 1000, 0.25);
    last = now;
    pollGamepad(app, pad);
    sizeCanvas(canvas);
    const host = app.frame(dt, canvas.width, canvas.height);
    const logs = app.take_logs();
    if (logs) {
      api.logs.push(...logs.split("\n"));
      if (debug) console.log(logs);
    }
    for (const line of host ? host.split("\n") : []) {
      const [cmd] = line.split(" ");
      if (cmd === "shot") api.shots.push(canvas.toDataURL("image/png"));
      if (cmd === "save-trace") api.logs.push(app.take_trace() ?? "");
    }
    debugPanel?.update();
    minimap.update(notes.spots());
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

start();
