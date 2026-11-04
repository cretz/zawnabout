import type { App } from "../pkg/zawnabout_web.js";

const MAP_PIXELS = 256;

/** A corner map of the whole world: you as an arrow, the soft edge as a dashed square and
 *  note pins in notes mode. Click it to make it bigger. */
export function setupMinimap(app: App, touch: boolean) {
  const canvas = document.getElementById("minimap") as HTMLCanvasElement;
  canvas.classList.remove("hidden");
  canvas.classList.toggle("top", touch);
  const ctx = canvas.getContext("2d")!;
  const [x0, z0, x1, z1, edge] = app.world_bounds();
  const span = x1 - x0;

  const image = new ImageData(new Uint8ClampedArray(app.map_image(MAP_PIXELS)), MAP_PIXELS, MAP_PIXELS);
  const base = document.createElement("canvas");
  base.width = base.height = MAP_PIXELS;
  base.getContext("2d")!.putImageData(image, 0, 0);

  // Small: a click makes it big. Big: a click picks a spot and asks to fly there.
  let big = false;
  let pick: { x: number; z: number } | null = null;
  const ask = document.createElement("div");
  ask.id = "travel";
  ask.className = "hidden";
  ask.innerHTML = `<button data-go="yes">Fly here</button><button data-go="no">Cancel</button>`;
  document.body.appendChild(ask);
  const close = document.createElement("button");
  close.id = "map-close";
  close.className = "round hidden";
  close.textContent = "✕";
  close.title = "Close map";
  document.body.appendChild(close);
  const setBig = (on: boolean) => {
    big = on;
    pick = null;
    canvas.classList.toggle("big", big);
    ask.classList.add("hidden");
    close.classList.toggle("hidden", !big);
    if (big) {
      // Sit the close button on the map's corner once it has grown.
      requestAnimationFrame(() => {
        const r = canvas.getBoundingClientRect();
        close.style.left = `${Math.min(r.right - 22, innerWidth - 48)}px`;
        close.style.top = `${Math.max(r.top - 22, 6)}px`;
      });
    }
  };
  canvas.addEventListener("click", (e) => {
    if (!big) {
      setBig(true);
      return;
    }
    const r = canvas.getBoundingClientRect();
    const u = (e.clientX - r.left) / r.width;
    const v = (e.clientY - r.top) / r.height;
    pick = { x: x0 + u * span, z: z0 + v * (z1 - z0) };
    ask.style.left = `${Math.min(e.clientX, innerWidth - 220)}px`;
    ask.style.top = `${Math.min(e.clientY + 14, innerHeight - 60)}px`;
    ask.classList.remove("hidden");
  });
  close.addEventListener("click", () => setBig(false));
  ask.addEventListener("click", (e) => {
    const go = (e.target as HTMLElement).dataset.go;
    if (!go) return;
    if (go === "yes" && pick) {
      // Arrive flying high over the spot, still facing the same way.
      const [, , heading] = app.map_marker();
      app.command(`goto ${pick.x.toFixed(0)} ${pick.z.toFixed(0)} 250 ${((heading * 180) / Math.PI).toFixed(0)}`);
      setBig(false);
    } else {
      pick = null;
      ask.classList.add("hidden");
    }
  });

  return {
    update(pins: number[]) {
      const size = Math.round(canvas.clientWidth * Math.min(devicePixelRatio || 1, 2));
      if (canvas.width !== size) canvas.width = canvas.height = size;
      const s = size / span;
      const px = (x: number) => (x - x0) * s;
      const pz = (z: number) => (z - z0) * s;
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(base, 0, 0, size, size);

      ctx.setLineDash([size / 60, size / 60]);
      ctx.strokeStyle = "rgba(255, 248, 236, 0.8)";
      ctx.lineWidth = Math.max(1, size / 200);
      ctx.strokeRect(px(x0 + edge), pz(z0 + edge), (span - 2 * edge) * s, (z1 - z0 - 2 * edge) * s);
      ctx.setLineDash([]);

      ctx.fillStyle = "#f2845c";
      for (let i = 0; i + 2 < pins.length; i += 3) {
        ctx.beginPath();
        ctx.arc(px(pins[i]), pz(pins[i + 2]), size / 90, 0, Math.PI * 2);
        ctx.fill();
      }

      if (pick) {
        ctx.beginPath();
        ctx.arc(px(pick.x), pz(pick.z), size / 40, 0, Math.PI * 2);
        ctx.fillStyle = "#fff8ec";
        ctx.strokeStyle = "#f2845c";
        ctx.lineWidth = Math.max(2, size / 100);
        ctx.fill();
        ctx.stroke();
      }

      // Heading 0 faces +z, which is down on the map.
      const [x, z, heading] = app.map_marker();
      const r = size / 28;
      ctx.save();
      ctx.translate(px(x), pz(z));
      ctx.rotate(-heading);
      ctx.beginPath();
      ctx.moveTo(0, r);
      ctx.lineTo(r * 0.65, -r * 0.7);
      ctx.lineTo(0, -r * 0.35);
      ctx.lineTo(-r * 0.65, -r * 0.7);
      ctx.closePath();
      ctx.fillStyle = "#f2845c";
      ctx.strokeStyle = "#fff8ec";
      ctx.lineWidth = Math.max(1.5, size / 120);
      ctx.fill();
      ctx.stroke();
      ctx.restore();
    },
  };
}
