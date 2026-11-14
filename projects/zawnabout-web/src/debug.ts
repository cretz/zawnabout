import type { App } from "../pkg/zawnabout_web.js";

interface Param {
  name: string;
  value: number;
  min: number;
  max: number;
  toggle: boolean;
  doc: string;
}

/** Live tuning panel: every tunable as a slider, a command box and a state readout. */
export function setupDebug(app: App, save: () => void) {
  const panel = document.getElementById("debug")!;
  const state = document.getElementById("debug-state")!;
  const out = document.getElementById("debug-out")!;
  const cmd = document.getElementById("debug-cmd") as HTMLInputElement;
  const list = document.getElementById("debug-params")!;
  panel.classList.remove("hidden");
  const open = new Set(["flight"]);

  const build = () => {
    const params: Param[] = JSON.parse(app.params_json());
    const changed = app.dump();
    list.innerHTML = "";
    const groups = new Map<string, HTMLDetailsElement>();
    for (const p of params) {
      const [group, field] = p.name.split(".");
      if (!groups.has(group)) {
        const d = document.createElement("details");
        d.open = open.has(group);
        d.addEventListener("toggle", () => (d.open ? open.add(group) : open.delete(group)));
        d.innerHTML = `<summary>${group}</summary>`;
        list.append(d);
        groups.set(group, d);
      }
      const row = document.createElement("div");
      row.className = "param";
      row.title = p.doc;
      row.classList.toggle("changed", changed.includes(`set ${p.name} `));
      const label = document.createElement("label");
      label.textContent = field;
      const input = document.createElement("input");
      const value = document.createElement("span");
      value.className = "value";
      const show = (v: number) => (value.textContent = p.toggle ? (v >= 0.5 ? "on" : "off") : String(+v.toFixed(3)));
      if (p.toggle) {
        input.type = "checkbox";
        input.checked = p.value >= 0.5;
      } else {
        input.type = "range";
        input.min = String(p.min);
        input.max = String(p.max);
        input.step = String((p.max - p.min) / 400);
        input.value = String(p.value);
      }
      show(p.value);
      input.addEventListener("input", () => {
        const v = p.toggle ? (input.checked ? 1 : 0) : Number(input.value);
        app.command(`set ${p.name} ${v}`);
        show(v);
        row.classList.add("changed");
        save();
      });
      row.append(label, input, value);
      groups.get(group)!.append(row);
    }
  };
  build();

  cmd.addEventListener("keydown", (e) => {
    if (e.key !== "Enter") return;
    out.textContent = app.command(cmd.value);
    cmd.value = "";
    build();
    save();
  });
  document.getElementById("debug-copy")!.addEventListener("click", () => {
    const dump = app.dump();
    navigator.clipboard?.writeText(dump);
    out.textContent = dump || "(all defaults)";
  });
  document.getElementById("debug-reset")!.addEventListener("click", () => {
    out.textContent = app.command("defaults");
    save();
    build();
  });
  document.getElementById("debug-hide")!.addEventListener("click", () => panel.classList.add("hidden"));

  let frames = 0;
  return {
    toggle: () => panel.classList.toggle("hidden"),
    update: () => {
      if (++frames % 10 !== 0 || panel.classList.contains("hidden")) return;
      state.textContent = app.state().replaceAll(" ", "\n");
    },
  };
}
