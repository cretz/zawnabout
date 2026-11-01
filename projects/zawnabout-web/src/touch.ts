import type { App } from "../pkg/zawnabout_web.js";

const RADIUS = 60;

/** A floating stick on the left half of the screen and action buttons on the right. */
export function setupTouch(app: App) {
  const root = document.getElementById("touch")!;
  const zone = document.getElementById("stick-zone")!;
  const base = document.getElementById("stick-base")!;
  const knob = document.getElementById("stick-knob")!;
  root.classList.remove("hidden");

  let stickId: number | null = null;
  let origin = { x: 0, y: 0 };
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
    base.style.left = `${e.clientX}px`;
    base.style.top = `${e.clientY}px`;
  });
  zone.addEventListener("pointermove", (e) => {
    if (e.pointerId !== stickId) return;
    let dx = e.clientX - origin.x;
    let dy = e.clientY - origin.y;
    const len = Math.hypot(dx, dy);
    if (len > RADIUS) {
      dx *= RADIUS / len;
      dy *= RADIUS / len;
    }
    knob.style.transform = `translate(${dx}px, ${dy}px)`;
    // Screen Y grows downward, the stick's up is +1.
    app.touch_stick(dx / RADIUS, -dy / RADIUS);
  });
  const end = (e: PointerEvent) => {
    if (e.pointerId !== stickId) return;
    stickId = null;
    rest();
  };
  zone.addEventListener("pointerup", end);
  zone.addEventListener("pointercancel", end);

  for (const button of root.querySelectorAll<HTMLButtonElement>("[data-action]")) {
    const action = button.dataset.action!;
    const set = (down: boolean) => {
      button.classList.toggle("down", down);
      app.touch_action(action, down);
    };
    button.addEventListener("pointerdown", (e) => {
      button.setPointerCapture(e.pointerId);
      set(true);
    });
    button.addEventListener("pointerup", () => set(false));
    button.addEventListener("pointercancel", () => set(false));
  }
}
