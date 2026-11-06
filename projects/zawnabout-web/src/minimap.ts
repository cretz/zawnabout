import type { App } from "../pkg/zawnabout_web.js";

const MAP_PIXELS = 256;
const ATLAS_PIXELS = 1024;

/** A legend group on the big map: its places' kinds, each with an icon and a name. */
interface Group {
  key: string;
  name: string;
  on: boolean;
  kinds: Record<string, [string, string]>;
}

const GROUPS: Group[] = [
  {
    key: "fun",
    name: "Fun and games",
    on: true,
    kinds: {
      zoo: ["🦓", "Zoo"],
      themepark: ["🎢", "Theme park"],
      waterpark: ["💦", "Water park"],
      ferris: ["🎡", "Fair"],
      circus: ["🎪", "Circus"],
      stadium: ["🏟️", "Stadium"],
      racetrack: ["🏁", "Racetrack"],
      golf: ["⛳", "Golf course"],
      amphitheater: ["🎭", "Open air theater"],
      cinema: ["🎬", "Cinema"],
      drivein: ["🎬", "Drive-in"],
      aquarium: ["🐠", "Aquarium"],
      maze: ["🌀", "Hedge maze"],
      bandstand: ["🎵", "Bandstand"],
      surfshack: ["🏄", "Surf shack"],
      hotspring: ["♨️", "Hot spring"],
      fountain: ["⛲", "Fountain"],
      skatepark: ["🛹", "Skate park"],
      pool: ["🏊", "Swimming pool"],
      bowling: ["🎳", "Bowling alley"],
      carnival: ["🎠", "Carnival"],
    },
  },
  {
    key: "outdoors",
    name: "Parks and outdoors",
    on: true,
    kinds: {
      park: ["🌳", "Park"],
      picnic: ["🧺", "Picnic spot"],
      campsite: ["⛺", "Campsite"],
      lookout: ["👀", "Lookout"],
      ropebridge: ["🌉", "Rope bridge"],
      observatory: ["🔭", "Observatory"],
      telescope: ["🔭", "Telescope"],
      treehouse: ["🌲", "Tree house"],
      beach: ["🏖️", "Beach"],
      restaurant: ["🍽️", "Summit restaurant"],
    },
  },
  {
    key: "shops",
    name: "Food and shops",
    on: true,
    kinds: {
      market: ["🍎", "Market"],
      grocery: ["🛒", "Grocery"],
      mall: ["🛍️", "Mall"],
      cafe: ["☕", "Cafe"],
      lemonade: ["🍋", "Lemonade stand"],
      gasstation: ["⛽", "Gas station"],
      icecream: ["🍦", "Ice cream"],
      diner: ["🍔", "Diner"],
    },
  },
  {
    key: "civic",
    name: "Town services",
    on: true,
    kinds: {
      hospital: ["🏥", "Hospital"],
      firestation: ["🚒", "Fire station"],
      police: ["🚓", "Police"],
      school: ["🏫", "School"],
      university: ["🎓", "University"],
      townhall: ["🏛️", "Town hall"],
      museum: ["🖼️", "Museum"],
      library: ["📚", "Library"],
      church: ["⛪", "Church"],
      postoffice: ["📮", "Post office"],
    },
  },
  {
    key: "travel",
    name: "Getting around",
    on: true,
    kinds: {
      station: ["🚉", "Station"],
      metro: ["🚇", "Subway"],
      "site:airport": ["✈️", "Airport"],
      "site:seaplane": ["🛩️", "Seaplane base"],
      "site:ferry": ["⛴️", "Ferry"],
      "site:marina": ["⛵", "Marina"],
      "site:docks": ["⚓", "Docks"],
      "site:bridge": ["🌉", "Bridge"],
      drawbridge: ["🌉", "Drawbridge"],
      port: ["🏗️", "Port"],
      "site:shipyard": ["🚢", "Shipyard"],
    },
  },
  {
    key: "work",
    name: "Farms and work",
    on: false,
    kinds: {
      farm: ["🚜", "Farm"],
      shroomfarm: ["🍄", "Mushroom farm"],
      mine: ["⛏️", "Mine"],
      solarfarm: ["☀️", "Solar farm"],
      turbine: ["💨", "Wind turbine"],
      hydro: ["💧", "Power house"],
      weir: ["💧", "Weir"],
      beehives: ["🐝", "Beehives"],
      watertower: ["🚰", "Water tower"],
    },
  },
  {
    key: "sights",
    name: "Sights",
    on: true,
    kinds: {
      "site:lighthouse": ["🗼", "Lighthouse"],
      "site:tower": ["🗼", "Tower"],
      "site:carving": ["🗿", "Cliff carving"],
      "site:wreck": ["⚓", "Shipwreck"],
      statue: ["🗽", "Statue"],
      sundial: ["🕰️", "Sundial"],
      stones: ["🪨", "Standing stones"],
      cairns: ["🪨", "Cairns"],
      "span-tower": ["🗼", "Span tower"],
    },
  },
];

/** Where each kind sits in the legend. */
const KIND_GROUP = new Map<string, Group>();
for (const g of GROUPS) for (const k of Object.keys(g.kinds)) KIND_GROUP.set(k, g);

interface Place {
  kind: string;
  id: string;
  x: number;
  z: number;
  under: boolean;
  icon: string;
  label: string;
  group: Group | null;
}

/** `home/terrace-hill` as `Terrace Hill`. */
function pretty(id: string): string {
  const last = id.split("/").pop() ?? id;
  return last
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

function readPlaces(app: App): Place[] {
  const raw = app.map_places().trim().split("\n");
  const places: Place[] = [];
  for (const line of raw) {
    const [kindRaw, id, xs, zs, under] = line.split(" ");
    if (!id) continue;
    const kind = kindRaw === "station" && id.includes("subway") ? "metro" : kindRaw;
    places.push({
      kind,
      id,
      x: Number(xs),
      z: Number(zs),
      under: under === "1" || kind === "metro",
      icon: "",
      label: "",
      group: KIND_GROUP.get(kind) ?? null,
    });
  }
  const towns = places.filter((p) => p.kind === "town" || p.kind === "village");
  for (const p of places) {
    if (p.group) {
      const [icon, name] = p.group.kinds[p.kind];
      p.icon = icon;
      // Name it by its town when it has one nearby, else by itself.
      let best: Place | null = null;
      let near = 900;
      for (const t of towns) {
        const d = Math.hypot(t.x - p.x, t.z - p.z);
        if (d < near) {
          near = d;
          best = t;
        }
      }
      p.label = best ? `${name}, ${pretty(best.id)}` : name;
      if (p.kind === "station" || p.kind === "metro") p.label = `${pretty(p.id)} ${p.kind === "metro" ? "subway" : ""}`.trim();
      if (p.kind.startsWith("site:")) p.label = `${name}, ${pretty(p.id)}`;
      if (p.under) p.label += " (in the caves)";
    } else {
      p.label = pretty(p.id);
    }
  }
  return places;
}

/** A corner map of the whole world: you as an arrow, the soft edge as a dashed square and
 *  note pins in notes mode. Click it to make it bigger, and bigger again for the atlas: the
 *  whole map over most of the screen with a legend of things to find, each shown by its
 *  icon and named on hover. */
export function setupMinimap(app: App, touch: boolean, pause: (on: boolean) => void) {
  const canvas = document.getElementById("minimap") as HTMLCanvasElement;
  canvas.classList.remove("hidden");
  canvas.classList.toggle("top", touch);
  const ctx = canvas.getContext("2d")!;
  const [x0, z0, x1, z1, edge] = app.world_bounds();
  const span = x1 - x0;

  const paint = (px: number) => {
    const image = new ImageData(new Uint8ClampedArray(app.map_image(px)), px, px);
    const c = document.createElement("canvas");
    c.width = c.height = px;
    c.getContext("2d")!.putImageData(image, 0, 0);
    return c;
  };
  const base = paint(MAP_PIXELS);
  let atlasBase: HTMLCanvasElement | null = null;
  const places = readPlaces(app);
  const LEGEND_KEY = "zawnabout.legend";
  let showCaves = true;
  let showTowns = true;
  try {
    const saved = JSON.parse(localStorage.getItem(LEGEND_KEY) ?? "null");
    if (saved) {
      for (const g of GROUPS) if (g.key in saved) g.on = !!saved[g.key];
      if ("caves" in saved) showCaves = !!saved.caves;
      if ("towns" in saved) showTowns = !!saved.towns;
    }
  } catch {
    // A fresh legend then.
  }
  const saveLegend = () => {
    const out: Record<string, boolean> = { caves: showCaves, towns: showTowns };
    for (const g of GROUPS) out[g.key] = g.on;
    try {
      localStorage.setItem(LEGEND_KEY, JSON.stringify(out));
    } catch {
      // Not saved, no matter.
    }
  };

  // Small, then big (a click picks a spot and asks to fly there), then the atlas.
  let size: "small" | "big" | "atlas" = "small";
  let pick: { x: number; z: number; label?: string } | null = null;
  let hover: Place | null = null;
  const ask = document.createElement("div");
  ask.id = "travel";
  ask.className = "hidden";
  ask.innerHTML = `<span id="travel-what"></span><button data-go="yes">Fly here</button><button data-go="no">Cancel</button>`;
  document.body.appendChild(ask);
  const close = document.createElement("button");
  close.id = "map-close";
  close.className = "round hidden";
  close.textContent = "✕";
  close.title = "Close map";
  document.body.appendChild(close);
  const grow = document.createElement("button");
  grow.id = "map-grow";
  grow.className = "round hidden";
  grow.textContent = "⤢";
  grow.title = "Open the big map with a legend";
  document.body.appendChild(grow);
  const tip = document.createElement("div");
  tip.id = "map-tip";
  tip.className = "hidden";
  document.body.appendChild(tip);
  const legend = document.createElement("div");
  legend.id = "map-legend";
  legend.className = "card hidden";
  document.body.appendChild(legend);

  const buildLegend = () => {
    const row = (key: string, on: boolean, icons: string, name: string) =>
      `<label><input type="checkbox" data-key="${key}" ${on ? "checked" : ""}/><span class="legend-icons">${icons}</span>${name}</label>`;
    let html = `<strong>Map</strong>`;
    html += row("towns", showTowns, "🏘️", "Towns and villages");
    for (const g of GROUPS) {
      const icons = [...new Set(Object.values(g.kinds).map(([i]) => i))].slice(0, 3).join("");
      html += row(g.key, g.on, icons, g.name);
    }
    html += row("caves", showCaves, "🔦", "Places down in the caves");
    html += `<p class="legend-hint">${touch ? "Tap" : "Hover over"} an icon to see what it is. ${touch ? "Tap" : "Click"} anywhere to fly there. ${touch ? "Pinch" : "Scroll"} to zoom, drag to look around.</p>`;
    legend.innerHTML = html;
  };
  buildLegend();
  legend.addEventListener("change", (e) => {
    const input = e.target as HTMLInputElement;
    const key = input.dataset.key;
    if (key === "caves") showCaves = input.checked;
    else if (key === "towns") showTowns = input.checked;
    else for (const g of GROUPS) if (g.key === key) g.on = input.checked;
    saveLegend();
  });

  const place = (el: HTMLElement, r: DOMRect) => {
    el.style.left = `${Math.min(r.right - 22, innerWidth - 48)}px`;
    el.style.top = `${Math.max(r.top - 22, 6)}px`;
  };
  const setSize = (to: typeof size) => {
    const was = size;
    size = to;
    pick = null;
    hover = null;
    canvas.classList.toggle("big", size === "big");
    canvas.classList.toggle("atlas", size === "atlas");
    ask.classList.add("hidden");
    tip.classList.add("hidden");
    close.classList.toggle("hidden", size === "small");
    grow.classList.toggle("hidden", size !== "big");
    legend.classList.toggle("hidden", size !== "atlas");
    if (size === "atlas" && !atlasBase) atlasBase = paint(ATLAS_PIXELS);
    if ((size === "atlas") !== (was === "atlas")) pause(size === "atlas");
    if (size !== "small") {
      // Sit the buttons on the map's corner once it has grown.
      requestAnimationFrame(() => {
        const r = canvas.getBoundingClientRect();
        place(close, r);
        grow.style.left = `${Math.min(r.right - 22, innerWidth - 48)}px`;
        grow.style.top = `${Math.max(r.top - 22, 6) + 48}px`;
      });
    }
  };

  // What the atlas shows: its top left corner and width in world meters. Wheel or pinch
  // to zoom, drag to pan. The smaller maps always show everything.
  const view = { x: x0, z: z0, s: span };
  const shownView = () => (size === "atlas" ? view : { x: x0, z: z0, s: span });
  const clampView = () => {
    view.s = Math.min(span, Math.max(span / 8, view.s));
    view.x = Math.min(x1 - view.s, Math.max(x0, view.x));
    view.z = Math.min(z1 - view.s, Math.max(z0, view.z));
  };
  // Map space from a point on the canvas.
  const toWorld = (clientX: number, clientY: number) => {
    const r = canvas.getBoundingClientRect();
    const u = (clientX - r.left) / r.width;
    const v = (clientY - r.top) / r.height;
    const w = shownView();
    return { x: w.x + u * w.s, z: w.z + v * w.s, r, u, v };
  };
  // Zoomed in, a sharp picture of just the part in view, drawn once zooming settles.
  let detail: { x: number; z: number; s: number; img: HTMLCanvasElement } | null = null;
  let settle = 0;
  const wantDetail = () => {
    clearTimeout(settle);
    settle = window.setTimeout(() => {
      if (size !== "atlas" || view.s > span * 0.6) return;
      const px = 512;
      const image = new ImageData(new Uint8ClampedArray(app.map_area(view.x, view.z, view.s, px)), px, px);
      const img = document.createElement("canvas");
      img.width = img.height = px;
      img.getContext("2d")!.putImageData(image, 0, 0);
      detail = { x: view.x, z: view.z, s: view.s, img };
    }, 250);
  };
  const zoomAt = (clientX: number, clientY: number, by: number) => {
    const at = toWorld(clientX, clientY);
    view.s *= by;
    clampView();
    view.x = at.x - at.u * view.s;
    view.z = at.z - at.v * view.s;
    clampView();
    wantDetail();
  };
  canvas.addEventListener(
    "wheel",
    (e) => {
      if (size !== "atlas") return;
      e.preventDefault();
      zoomAt(e.clientX, e.clientY, Math.pow(1.0015, e.deltaY));
    },
    { passive: false },
  );
  // Dragging pans, two fingers pinch. A drag is not a click.
  const touches = new Map<number, { x: number; y: number }>();
  let dragged = 0;
  canvas.addEventListener("pointerdown", (e) => {
    if (size !== "atlas") return;
    touches.set(e.pointerId, { x: e.clientX, y: e.clientY });
    dragged = 0;
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    const was = touches.get(e.pointerId);
    if (size !== "atlas" || !was) return;
    const r = canvas.getBoundingClientRect();
    if (touches.size >= 2) {
      const [a, b] = [...touches.values()];
      const before = Math.hypot(a.x - b.x, a.y - b.y);
      touches.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const [c, d] = [...touches.values()];
      const after = Math.hypot(c.x - d.x, c.y - d.y);
      if (before > 10 && after > 10) zoomAt((c.x + d.x) / 2, (c.y + d.y) / 2, before / after);
      dragged = 99;
      return;
    }
    const dx = e.clientX - was.x;
    const dy = e.clientY - was.y;
    touches.set(e.pointerId, { x: e.clientX, y: e.clientY });
    dragged += Math.abs(dx) + Math.abs(dy);
    if (dragged > 6 && view.s < span) {
      view.x -= (dx / r.width) * view.s;
      view.z -= (dy / r.height) * view.s;
      clampView();
      wantDetail();
    }
  });
  const lift = (e: PointerEvent) => touches.delete(e.pointerId);
  canvas.addEventListener("pointerup", lift);
  canvas.addEventListener("pointercancel", lift);
  const shown = (p: Place) => {
    if (p.under && !showCaves) return false;
    if (p.kind === "town" || p.kind === "village") return showTowns;
    return !!p.group?.on;
  };
  // The icon under a point on the atlas, if any.
  const iconAt = (clientX: number, clientY: number): Place | null => {
    if (size !== "atlas") return null;
    const { x, z, r } = toWorld(clientX, clientY);
    const reach = (shownView().s / r.width) * (touch ? 22 : 14);
    let best: Place | null = null;
    let near = reach;
    for (const p of places) {
      if (!p.group || !shown(p)) continue;
      const d = Math.hypot(p.x - x, p.z - z);
      if (d < near) {
        near = d;
        best = p;
      }
    }
    return best;
  };
  const showTip = (p: Place | null, clientX: number, clientY: number) => {
    hover = p;
    if (!p) {
      tip.classList.add("hidden");
      return;
    }
    tip.textContent = `${p.icon} ${p.label}`;
    tip.style.left = `${Math.min(clientX + 14, innerWidth - 240)}px`;
    tip.style.top = `${Math.max(clientY - 34, 6)}px`;
    tip.classList.remove("hidden");
  };
  canvas.addEventListener("pointermove", (e) => {
    if (size === "atlas" && e.pointerType === "mouse") showTip(iconAt(e.clientX, e.clientY), e.clientX, e.clientY);
  });
  canvas.addEventListener("pointerleave", () => {
    if (size === "atlas") showTip(null, 0, 0);
  });
  canvas.addEventListener("click", (e) => {
    if (size === "small") {
      setSize("big");
      return;
    }
    if (size === "atlas" && dragged > 6) return;
    const hit = iconAt(e.clientX, e.clientY);
    const at = hit ? { x: hit.x, z: hit.z } : toWorld(e.clientX, e.clientY);
    pick = { x: at.x, z: at.z, label: hit?.label };
    if (hit) showTip(hit, e.clientX, e.clientY);
    (ask.querySelector("#travel-what") as HTMLElement).textContent = hit ? `${hit.icon} ${hit.label}` : "";
    ask.style.left = `${Math.min(e.clientX, innerWidth - 300)}px`;
    ask.style.top = `${Math.min(e.clientY + 14, innerHeight - 60)}px`;
    ask.classList.remove("hidden");
  });
  close.addEventListener("click", () => setSize("small"));
  grow.addEventListener("click", () => setSize("atlas"));
  ask.addEventListener("click", (e) => {
    const go = (e.target as HTMLElement).dataset.go;
    if (!go) return;
    if (go === "yes" && pick) {
      // Arrive flying high over the spot, still facing the same way.
      const [, , heading] = app.map_marker();
      app.command(`goto ${pick.x.toFixed(0)} ${pick.z.toFixed(0)} 250 ${((heading * 180) / Math.PI).toFixed(0)}`);
      setSize("small");
    } else {
      pick = null;
      ask.classList.add("hidden");
    }
  });
  window.addEventListener("keydown", (e) => {
    if (e.code === "KeyM" && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) {
      setSize(size === "atlas" ? "small" : "atlas");
      e.stopImmediatePropagation();
    } else if (e.code === "Escape" && size === "atlas") {
      setSize("small");
      e.stopImmediatePropagation();
    }
  }, true);

  return {
    /** Whether the atlas is open, so the game stays paused under it. */
    atlas: () => size === "atlas",
    update(pins: number[]) {
      const px = Math.round(canvas.clientWidth * Math.min(devicePixelRatio || 1, 2));
      if (canvas.width !== px) canvas.width = canvas.height = px;
      const w = shownView();
      const s = px / w.s;
      const mx = (x: number) => (x - w.x) * s;
      const mz = (z: number) => (z - w.z) * s;
      ctx.imageSmoothingEnabled = true;
      const img = size === "atlas" && atlasBase ? atlasBase : base;
      const k = img.width / span;
      ctx.drawImage(img, (w.x - x0) * k, (w.z - z0) * k, w.s * k, w.s * k, 0, 0, px, px);
      // The sharp close-up over it, where it covers the view.
      if (size === "atlas" && detail && view.s <= span * 0.6) {
        const d = detail;
        ctx.drawImage(d.img, (d.x - w.x) * s, (d.z - w.z) * s, d.s * s, d.s * s);
      }

      ctx.setLineDash([px / 60, px / 60]);
      ctx.strokeStyle = "rgba(255, 248, 236, 0.8)";
      ctx.lineWidth = Math.max(1, px / 200);
      ctx.strokeRect(mx(x0 + edge), mz(z0 + edge), (span - 2 * edge) * s, (z1 - z0 - 2 * edge) * s);
      ctx.setLineDash([]);

      if (size === "atlas") {
        const dpr = px / canvas.clientWidth;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        if (showTowns) {
          ctx.font = `800 ${Math.round(13 * dpr)}px ui-rounded, "Segoe UI", system-ui, sans-serif`;
          ctx.lineWidth = 3 * dpr;
          ctx.strokeStyle = "rgba(74, 59, 82, 0.75)";
          ctx.fillStyle = "#fff8ec";
          for (const p of places) {
            if ((p.kind !== "town" && p.kind !== "village") || (p.under && !showCaves)) continue;
            const name = p.label + (p.under ? " (caves)" : "");
            ctx.strokeText(name, mx(p.x), mz(p.z) - 16 * dpr);
            ctx.fillText(name, mx(p.x), mz(p.z) - 16 * dpr);
          }
        }
        ctx.font = `${Math.round(16 * dpr)}px "Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", sans-serif`;
        for (const p of places) {
          if (!p.group || !shown(p)) continue;
          const big = p === hover;
          ctx.beginPath();
          ctx.arc(mx(p.x), mz(p.z), (big ? 15 : 12) * dpr, 0, Math.PI * 2);
          ctx.fillStyle = p.under ? "rgba(58, 44, 70, 0.85)" : "rgba(255, 248, 236, 0.9)";
          ctx.fill();
          ctx.fillText(p.icon, mx(p.x), mz(p.z) + dpr);
        }
      }

      ctx.fillStyle = "#f2845c";
      for (let i = 0; i + 2 < pins.length; i += 3) {
        ctx.beginPath();
        ctx.arc(mx(pins[i]), mz(pins[i + 2]), px / 90, 0, Math.PI * 2);
        ctx.fill();
      }

      if (pick) {
        ctx.beginPath();
        ctx.arc(mx(pick.x), mz(pick.z), px / (size === "atlas" ? 70 : 40), 0, Math.PI * 2);
        ctx.fillStyle = "#fff8ec";
        ctx.strokeStyle = "#f2845c";
        ctx.lineWidth = Math.max(2, px / 100);
        ctx.fill();
        ctx.stroke();
      }

      // Heading 0 faces +z, which is down on the map.
      const [x, z, heading] = app.map_marker();
      const r = px / (size === "atlas" ? 60 : 28);
      ctx.save();
      ctx.translate(mx(x), mz(z));
      ctx.rotate(-heading);
      ctx.beginPath();
      ctx.moveTo(0, r);
      ctx.lineTo(r * 0.65, -r * 0.7);
      ctx.lineTo(0, -r * 0.35);
      ctx.lineTo(-r * 0.65, -r * 0.7);
      ctx.closePath();
      ctx.fillStyle = "#f2845c";
      ctx.strokeStyle = "#fff8ec";
      ctx.lineWidth = Math.max(1.5, px / 120);
      ctx.fill();
      ctx.stroke();
      ctx.restore();
    },
  };
}
