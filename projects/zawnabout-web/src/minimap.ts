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

  let big = false;
  canvas.addEventListener("click", () => {
    big = !big;
    canvas.classList.toggle("big", big);
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
