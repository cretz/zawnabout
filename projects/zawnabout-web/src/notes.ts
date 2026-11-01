import type { App } from "../pkg/zawnabout_web.js";

/** A design note, as stored in `world/feedback/<id>.json`. */
interface Note {
  id?: string;
  created?: string;
  status?: "open" | "done";
  text: string;
  spot: number[];
  view: number[];
  near: string[];
  replies?: { from: string; text: string; at?: string }[];
}

const LOCAL_KEY = "zawnabout.notes";

/** Where notes go: the dev server writes them into the repo, otherwise they stay in this
 *  browser and can be downloaded. Nothing is ever sent anywhere else. */
class Store {
  server = false;

  async load(): Promise<Note[]> {
    try {
      const res = await fetch("/__notes");
      if (res.ok && res.headers.get("content-type")?.includes("json")) {
        this.server = true;
        return await res.json();
      }
    } catch {
      // No dev server, so keep notes locally.
    }
    this.server = false;
    return this.local();
  }

  async save(note: Note): Promise<Note> {
    if (this.server) {
      const res = await fetch("/__notes", { method: "POST", body: JSON.stringify(note) });
      if (res.ok) return await res.json();
      this.server = false;
    }
    const all = this.local();
    note.id ??= `local-${Date.now()}`;
    note.created ??= new Date().toISOString();
    note.status ??= "open";
    note.replies ??= [];
    all.push(note);
    this.setLocal(all);
    return note;
  }

  async remove(id: string) {
    if (this.server) {
      await fetch(`/__notes/${id}`, { method: "DELETE" });
      return;
    }
    this.setLocal(this.local().filter((n) => n.id !== id));
  }

  private local(): Note[] {
    try {
      return JSON.parse(localStorage.getItem(LOCAL_KEY) ?? "[]");
    } catch {
      return [];
    }
  }

  private setLocal(notes: Note[]) {
    try {
      localStorage.setItem(LOCAL_KEY, JSON.stringify(notes));
    } catch {
      // Storage can be off in private windows. The download button still works.
    }
  }
}

const esc = (s: string) =>
  s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

/** Notes mode: freeze the world, fly a free camera, click a spot and leave a note there. */
export function setupNotes(app: App, canvas: HTMLCanvasElement, touch: boolean) {
  const $ = (id: string) => document.getElementById(id)!;
  const bar = $("notes-bar");
  const panel = $("notes-panel");
  const list = $("notes-list");
  const where = $("notes-where");
  const download = $("notes-download") as HTMLButtonElement;
  const dialog = $("note-dialog");
  const nearText = $("note-near");
  const text = $("note-text") as HTMLTextAreaElement;
  const store = new Store();
  let notes: Note[] = [];
  let draft: { spot: number[]; view: number[]; near: string[] } | null = null;

  $("notes-help").textContent = touch
    ? "Stick moves, drag looks, Flap rises, Dive sinks. Tap a spot to leave a note."
    : "WASD moves, arrows or drag look, Space rises, Shift sinks. Click a spot to leave a note. N or Esc goes back.";

  const pins = () => {
    const xyz = new Float32Array(notes.flatMap((n) => n.spot));
    const done = Uint8Array.from(notes, (n) => (n.status === "done" ? 1 : 0));
    app.set_pins(xyz, done);
  };

  const render = () => {
    where.textContent = store.server ? "saved to world/feedback" : "kept in this browser";
    download.classList.toggle("hidden", store.server || notes.length === 0);
    list.innerHTML = "";
    if (notes.length === 0) {
      list.innerHTML = `<p class="notes-empty">No notes yet. ${touch ? "Tap" : "Click"} anywhere in the world to leave one.</p>`;
    }
    for (const n of [...notes].reverse()) {
      const item = document.createElement("div");
      item.className = `note ${n.status === "done" ? "done" : ""}`;
      const replies = (n.replies ?? [])
        .map((r) => `<div class="reply"><b>${esc(r.from)}</b> ${esc(r.text)}</div>`)
        .join("");
      item.innerHTML = `
        <div class="note-text">${esc(n.text)}</div>
        <div class="note-meta">${n.status === "done" ? "done" : "open"} · ${esc(n.near.filter((s) => s.startsWith("in ")).pop() ?? n.near[0] ?? "")}</div>
        ${replies}
        <div class="note-buttons"><button class="go">Go there</button><button class="del">Delete</button></div>`;
      item.querySelector(".go")!.addEventListener("click", () => {
        const [x, y, z, yaw, pitch] = n.view;
        app.command(`view ${x} ${y} ${z} ${yaw} ${pitch}`);
      });
      item.querySelector(".del")!.addEventListener("click", async () => {
        if (!n.id || !confirm("Delete this note?")) return;
        await store.remove(n.id);
        notes = notes.filter((m) => m !== n);
        pins();
        render();
      });
      list.append(item);
    }
  };

  const refresh = async () => {
    notes = await store.load();
    pins();
    render();
  };

  // Tap the title to fold the list away, handy on small screens.
  panel.querySelector(".notes-head strong")!.addEventListener("click", () => panel.classList.toggle("collapsed"));

  download.addEventListener("click", () => {
    const blob = new Blob([JSON.stringify(notes, null, 2)], { type: "application/json" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = "zawnabout-notes.json";
    a.click();
    URL.revokeObjectURL(a.href);
  });

  const closeDialog = () => {
    dialog.classList.add("hidden");
    draft = null;
    pins();
  };
  const saveDraft = async () => {
    const body = text.value.trim();
    if (!draft || !body) return;
    const saved = await store.save({ ...draft, text: body });
    notes.push(saved);
    closeDialog();
    render();
  };
  $("note-save").addEventListener("click", saveDraft);
  $("note-cancel").addEventListener("click", closeDialog);
  text.addEventListener("keydown", (e) => {
    // Ctrl or Cmd with Enter saves, Escape cancels.
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) saveDraft();
    if (e.key === "Escape") closeDialog();
    e.stopPropagation();
  });

  // Load once up front so the map shows pins before notes mode is opened.
  refresh();

  const set = (on: boolean) => {
    app.notes(on);
    bar.classList.toggle("hidden", !on);
    panel.classList.toggle("hidden", !on);
    $("notes-btn").classList.toggle("on", on);
    if (on) refresh();
    else closeDialog();
  };

  const pick = (clientX: number, clientY: number) => {
    const spot = app.notes_pick(clientX / canvas.clientWidth, clientY / canvas.clientHeight);
    if (!spot) return;
    draft = JSON.parse(spot);
    nearText.textContent = draft!.near.slice(0, 3).join(" · ");
    text.value = "";
    dialog.classList.remove("hidden");
    text.focus();
  };

  // Drag to look, click to pin. Pointer capture keeps a drag going off the canvas.
  let drag: { x: number; y: number; moved: number; id: number } | null = null;
  canvas.addEventListener("pointerdown", (e) => {
    if (!app.notes_active()) return;
    drag = { x: e.clientX, y: e.clientY, moved: 0, id: e.pointerId };
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    drag.moved += Math.abs(dx) + Math.abs(dy);
    drag.x = e.clientX;
    drag.y = e.clientY;
    app.notes_look(dx, dy);
  });
  canvas.addEventListener("pointerup", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    if (drag.moved < 8 && dialog.classList.contains("hidden")) pick(e.clientX, e.clientY);
    drag = null;
  });

  $("notes-btn").addEventListener("click", () => set(!app.notes_active()));
  $("notes-exit").addEventListener("click", () => set(false));

  return {
    /// Every note's spot, x y z flattened, for the map.
    spots: () => notes.flatMap((n) => n.spot),
    active: () => app.notes_active(),
    toggle: () => set(!app.notes_active()),
    writing: () => !dialog.classList.contains("hidden"),
    pick,
  };
}
