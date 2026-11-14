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
