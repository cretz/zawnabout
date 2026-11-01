import init, { App } from "../pkg/zawnabout_web.js";
import { setupDebug } from "./debug.ts";
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
        ["Brake", "Hold to slow down and land"],
      ]
    : [
        ["<kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd> or arrows", "Steer, walk"],
        ["<kbd>Space</kbd>", "Flap (tap faster to flap harder)"],
        ["<kbd>Shift</kbd>", "Tuck wings and dive"],
        ["<kbd>E</kbd> or <kbd>X</kbd>", "Brake and land"],
        ["Gamepad", "Stick, A to flap, triggers"],
      ];
  return rows.map(([k, v]) => `<span class="keys">${k}</span><span>${v}</span>`).join("");
}

function sizeCanvas(canvas: HTMLCanvasElement) {
  // Cap the pixel ratio so tablets stay smooth.
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
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
  if (touch) setupTouch(app);

  const welcome = $("welcome");
  $("controls").innerHTML = controlsHtml(touch);
  if (touch) $("start-hint").textContent = "Tap to start";
  const showHelp = (show: boolean) => welcome.classList.toggle("hidden", !show);
  if (!query.has("nowelcome")) showHelp(true);
  $("help-btn").addEventListener("click", () => showHelp(welcome.classList.contains("hidden")));
  welcome.addEventListener("pointerdown", () => showHelp(false));

  const typing = (e: Event) => e.target instanceof HTMLInputElement;
  window.addEventListener("keydown", (e) => {
    if (typing(e)) return;
    if (e.code === "Backquote" && debugPanel) {
      debugPanel.toggle();
      return;
    }
    if (e.code === "KeyH" || e.code === "Escape") {
      showHelp(welcome.classList.contains("hidden"));
      return;
    }
    showHelp(false);
    if (GAME_KEYS.has(e.code)) e.preventDefault();
    app.key(e.code, true, e.repeat);
  });
  window.addEventListener("keyup", (e) => {
    if (typing(e)) return;
    app.key(e.code, false, false);
  });
  window.addEventListener("blur", () => app.blur());
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
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

start();
