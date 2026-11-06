import init, { App } from "../pkg/zawnabout_web.js";
import { setupDebug } from "./debug.ts";
import { setupMinimap } from "./minimap.ts";
import { setupNotes } from "./notes.ts";
import { setupSound } from "./sound.ts";
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
        ["<kbd>I</kbd><kbd>J</kbd><kbd>K</kbd><kbd>L</kbd> or right drag", "Look around"],
        ["<kbd>M</kbd>", "Big map with everything to find"],
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

  const welcome = $("welcome");
  const notes = setupNotes(app, canvas, touch);
  setupSound(app);
  // The atlas pauses the world while it is open, unless the pause card already has.
  const minimap = setupMinimap(app, touch, (on) => {
    if (welcome.classList.contains("hidden")) app.command(on ? "pause" : "resume");
  });
  // A tap on the touch stick area picks a note spot in notes mode, else a landing spot.
  if (touch)
    setupTouch(app, (x, y) => {
      if (!notes.active()) app.tap(x / innerWidth, y / innerHeight);
      else if (!notes.writing()) notes.pick(x, y);
    });

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
    if (show || !minimap.atlas()) app.command(show ? "pause" : "resume");
  };
  if (!query.has("nowelcome")) showHelp(true);
  else started = true;
  // Sound on or off from the pause card, remembered, and kept in step with the U key.
  const SOUND_KEY = "zawnabout.sound";
  const soundBox = $<HTMLInputElement>("sound-on");
  try {
    soundBox.checked = localStorage.getItem(SOUND_KEY) !== "0";
  } catch {
    soundBox.checked = true;
  }
  app.command(soundBox.checked ? "sound on" : "sound off");
  soundBox.parentElement!.addEventListener("pointerdown", (e) => e.stopPropagation());
  soundBox.addEventListener("change", () => {
    app.command(soundBox.checked ? "sound on" : "sound off");
    try {
      localStorage.setItem(SOUND_KEY, soundBox.checked ? "1" : "0");
    } catch {
      // Kept for this visit only.
    }
  });
  addEventListener("keyup", (e) => {
    if (e.code !== "KeyU") return;
    soundBox.checked = !app.muted();
    try {
      localStorage.setItem(SOUND_KEY, soundBox.checked ? "1" : "0");
    } catch {
      // Kept for this visit only.
    }
  });
  // Leaving the tab or the window pauses too, unless turned off on the pause card.
  const PAUSE_KEY = "zawnabout.pauseaway";
  const away = $<HTMLInputElement>("pause-away");
  try {
    away.checked = localStorage.getItem(PAUSE_KEY) !== "0";
  } catch {
    away.checked = true;
  }
  away.parentElement!.addEventListener("pointerdown", (e) => e.stopPropagation());
  away.addEventListener("change", () => {
    try {
      localStorage.setItem(PAUSE_KEY, away.checked ? "1" : "0");
    } catch {
      // Kept for this visit only.
    }
  });
  const leave = () => {
    if (away.checked && started && !notes.active() && !query.has("nowelcome")) showHelp(true);
  };
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) leave();
  });
  window.addEventListener("blur", leave);

  // Changing keys on the pause card: pick an action, press its new key. Kept in the
  // browser for next time.
  const KEYS_SAVED = "zawnabout.keys";
  try {
    const saved = localStorage.getItem(KEYS_SAVED);
    if (saved) app.set_bindings_text(saved);
  } catch {
    // The game's own keys then.
  }
  let rebinding: string | null = null;
  const LABELS: Record<string, string> = {
    up: "Nose up",
    down: "Nose down",
    left: "Turn left",
    right: "Turn right",
    flap: "Flap",
    tuck: "Dive",
    brake: "Brake, land, hover",
    breath: "Breathe fire",
  };
  const keyName = (code: string) =>
    code.replace(/^Key|^Digit/, "").replace(/^Arrow(.*)/, "$1 arrow").replace(/(Left|Right)$/, "");
  // The corner card shows the keys as they are now, changed or not.
  const drawCorner = () => {
    const bound = new Map(
      app
        .bindings_text()
        .trim()
        .split(/\r?\n/)
        .map((line) => {
          const [action, ...codes] = line.split(" ");
          return [action, codes.filter((c) => !c.startsWith("Arrow"))] as [string, string[]];
        }),
    );
    const kbd = (action: string, n: number) =>
      [...new Set((bound.get(action) ?? []).map(keyName))]
        .slice(0, n)
        .map((k) => `<kbd>${k}</kbd>`)
        .join(" ");
    $("keys-list").innerHTML = [
      [["up", "left", "down", "right"].map((a) => kbd(a, 1)).join(""), "Steer"],
      [kbd("flap", 1), "Flap"],
      [kbd("tuck", 1), "Dive"],
      [kbd("brake", 2), "Brake, land, hover"],
      [kbd("breath", 1), "Breathe fire"],
      ["<kbd>I</kbd><kbd>J</kbd><kbd>K</kbd><kbd>L</kbd>", "Look around"],
      ["<kbd>M</kbd>", "Big map"],
      ["<kbd>N</kbd>", "Notes"],
      ["<kbd>U</kbd>", "Sound on/off"],
      ["<kbd>Esc</kbd>", "Pause, all controls"],
    ]
      .map(([k, v]) => `<span class="keys">${k}</span><span>${v}</span>`)
      .join("");
  };
  const drawKeys = () => {
    drawCorner();
    $("rebind-list").innerHTML = app
      .bindings_text()
      .trim()
      .split(/\r?\n/)
      .map((line) => {
        const [action, ...codes] = line.split(" ");
        const shown =
          rebinding === action ? "Press a key..." : [...new Set(codes.map(keyName))].join(", ") || "none";
        return `<span>${LABELS[action] ?? action}</span><button type="button" data-action="${action}">${shown}</button>`;
      })
      .join("");
  };
  const saveKeys = () => {
    try {
      localStorage.setItem(KEYS_SAVED, app.bindings_text());
    } catch {
      // Kept for this visit only.
    }
  };
  if (!touch) {
    const rebind = $("rebind");
    rebind.classList.remove("hidden");
    rebind.addEventListener("pointerdown", (e) => e.stopPropagation());
    $("rebind-list").addEventListener("click", (e) => {
      const action = (e.target as HTMLElement).dataset?.action;
      if (action) {
        rebinding = action;
        drawKeys();
      }
    });
    $("rebind-reset").addEventListener("click", () => {
      app.reset_keys();
      saveKeys();
      drawKeys();
    });
    // While waiting for a new key, that key goes nowhere else.
    window.addEventListener(
      "keydown",
      (e) => {
        if (!rebinding) return;
        e.preventDefault();
        e.stopImmediatePropagation();
        if (e.code !== "Escape") {
          app.bind_key(rebinding, e.code);
          saveKeys();
        }
        rebinding = null;
        drawKeys();
      },
      { capture: true },
    );
    drawKeys();
  }

  // A small corner card of the keys for keyboard players, folded away with its button.
  const keys = $("keys-card");
  if (!touch) {
    const KEYS_KEY = "zawnabout.keyscard";
    drawCorner();
    const fold = (open: boolean) => {
      keys.classList.toggle("folded", !open);
      try {
        localStorage.setItem(KEYS_KEY, open ? "1" : "0");
      } catch {
        // Kept for this visit only.
      }
    };
    let open = true;
    try {
      open = localStorage.getItem(KEYS_KEY) !== "0";
    } catch {
      // Open then.
    }
    keys.classList.remove("hidden");
    fold(open);
    $("keys-toggle").addEventListener("click", () => fold(keys.classList.contains("folded")));
  }
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
    // U sets the sound, even with the card open.
    if (e.code === "KeyU") {
      if (!e.repeat) app.key(e.code, true, false);
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
  // Click or tap the world to fly there and land. Drag it, or hold the right button, to
  // look around.
  let press: { x: number; y: number; at: number; id: number; drag: boolean } | null = null;
  canvas.addEventListener("pointerdown", (e) => {
    // The right button, or Ctrl with a click on a Mac, looks round from the first move.
    const look = e.pointerType === "mouse" && (e.button === 2 || (e.button === 0 && e.ctrlKey));
    press = { x: e.clientX, y: e.clientY, at: performance.now(), id: e.pointerId, drag: look };
    if (look) canvas.setPointerCapture(e.pointerId);
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
    minimap.update(notes.active() ? notes.spots() : []);
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

start();
