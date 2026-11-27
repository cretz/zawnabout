import type { App } from "../pkg/zawnabout_web.js";

const RADIUS = 80;
/** Small wobbles of the thumb near the middle do nothing. */
const DEAD = 0.08;

/** Softens the middle of the stick so small moves make small corrections, and full tilt
 *  is still full. */
function response(dx: number, dy: number): [number, number] {
  const len = Math.hypot(dx, dy);
  if (len <= DEAD) return [0, 0];
  const t = Math.min((len - DEAD) / (1 - DEAD), 1);
  const curved = t * (0.35 + 0.65 * t);
  const k = curved / len;
  return [dx * k, dy * k];
}

/** A floating stick on the left half of the screen and action buttons on the right. A
 *  quick tap on the stick area calls `onTap` with page coordinates. */
export function setupTouch(app: App, onTap: (x: number, y: number) => void) {
  const root = document.getElementById("touch")!;
  const zone = document.getElementById("stick-zone")!;
  const base = document.getElementById("stick-base")!;
  const knob = document.getElementById("stick-knob")!;
  root.classList.remove("hidden");

  let stickId: number | null = null;
  let origin = { x: 0, y: 0 };
  let downAt = 0;
  let moved = 0;
  const rest = () => {
    base.style.left = "";
    base.style.top = "";
    knob.style.transform = "";
    app.touch_stick(0, 0);
  };

  zone.addEventListener("pointerdown", (e) => {
    if (stickId !== null) return;
    stickId = e.pointerId;
    zone.setPointerCapture(e.pointerId);
    origin = { x: e.clientX, y: e.clientY };
    downAt = performance.now();
    moved = 0;
    base.style.left = `${e.clientX}px`;
    base.style.top = `${e.clientY}px`;
  });
  zone.addEventListener("pointermove", (e) => {
    if (e.pointerId !== stickId) return;
    let dx = e.clientX - origin.x;
    let dy = e.clientY - origin.y;
    const len = Math.hypot(dx, dy);
    moved = Math.max(moved, len);
    if (len > RADIUS) {
      dx *= RADIUS / len;
      dy *= RADIUS / len;
    }
    knob.style.transform = `translate(${dx}px, ${dy}px)`;
    // Screen Y grows downward, the stick's up is +1.
    const [sx, sy] = response(dx / RADIUS, -dy / RADIUS);
    app.touch_stick(sx, sy);
  });
  const end = (e: PointerEvent) => {
    if (e.pointerId !== stickId) return;
    stickId = null;
    rest();
    // A quick tap that never became a drag picks a landing spot.
    if (e.type === "pointerup" && moved < 12 && performance.now() - downAt < 300) {
      onTap(e.clientX, e.clientY);
    }
  };
  zone.addEventListener("pointerup", end);
  zone.addEventListener("pointercancel", end);

  for (const button of root.querySelectorAll<HTMLButtonElement>("[data-action]")) {
    const action = button.dataset.action!;
    const set = (down: boolean) => {
      button.classList.toggle("down", down);
      app.touch_action(action, down);
    };
    // Toggle buttons stay on until tapped again, so a thumb is free for flying.
    if (button.dataset.toggle !== undefined) {
      let on = false;
      button.addEventListener("pointerdown", () => {
        on = !on;
        set(on);
      });
      continue;
    }
    button.addEventListener("pointerdown", (e) => {
      button.setPointerCapture(e.pointerId);
      set(true);
    });
    button.addEventListener("pointerup", () => set(false));
    button.addEventListener("pointercancel", () => set(false));
  }
}

/** Gamepad buttons for each screen button, as the game binds them. */
const PAD_BUTTONS: Record<string, number[]> = {
  flap: [0],
  brake: [1, 3, 4, 6],
  breath: [2],
  tuck: [5, 7],
};

/** Shows a gamepad on the screen controls: the knob follows the left stick and a held
 *  button lights its screen button, so the two always agree. A thumb on the screen
 *  stick wins. */
export function mirrorPad(axes: ArrayLike<number>, buttons: ArrayLike<number>) {
  const root = document.getElementById("touch");
  if (!root || root.classList.contains("hidden")) return;
  const knob = document.getElementById("stick-knob");
  const base = document.getElementById("stick-base");
  if (knob && base && !base.style.left) {
    let x = axes[0] ?? 0;
    let y = axes[1] ?? 0;
    const len = Math.hypot(x, y);
    if (len > 1) {
      x /= len;
      y /= len;
    }
    knob.style.transform = len > 0.1 ? `translate(${x * RADIUS}px, ${y * RADIUS}px)` : "";
  }
  for (const button of root.querySelectorAll<HTMLButtonElement>("[data-action]")) {
    const held = (PAD_BUTTONS[button.dataset.action!] ?? []).some((b) => (buttons[b] ?? 0) > 0.5);
    button.classList.toggle("pad-down", held);
  }
}
