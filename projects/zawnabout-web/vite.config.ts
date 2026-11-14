import { mkdirSync, readdirSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import type { IncomingMessage } from "node:http";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

// Design notes left in notes mode live in the repo, one JSON file each, so the agent can
// read them and reply. Only the dev server has this endpoint: a static build has no
// server, and the page falls back to keeping notes in the browser.
const NOTES = join(dirname(fileURLToPath(import.meta.url)), "../../../working-area/feedback");
// Perf reports sent from `?perf`, one JSON file each, for the agent to read.
const PERF = join(dirname(fileURLToPath(import.meta.url)), "../../../working-area/perf");
const ID = /^[0-9]{8}-[0-9]{6}-[a-z0-9]{4}$/;

function body(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let data = "";
    req.on("data", (c) => (data += c));
    req.on("end", () => resolve(data));
    req.on("error", reject);
  });
}

// Whether a request comes from the game's own page. Any other page open in the same
// browser can send a plain POST here without asking, and only its origin tells it apart.
function sameOrigin(req: IncomingMessage): boolean {
  const origin = req.headers.origin;
  if (!origin) return true;
  try {
    return new URL(origin).host === req.headers.host;
  } catch {
    return false;
  }
}

function newId(): string {
  const d = new Date();
  const p = (n: number, w = 2) => String(n).padStart(w, "0");
  const date = `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}`;
  const time = `${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}`;
  return `${date}-${time}-${Math.random().toString(36).slice(2, 6).padEnd(4, "0")}`;
}

function notesPlugin(): Plugin {
  return {
    name: "zawnabout-notes",
    configureServer(server) {
      server.middlewares.use("/__perf", async (req, res) => {
        res.setHeader("content-type", "application/json");
        if (req.method !== "POST" || !sameOrigin(req)) {
          res.statusCode = 400;
          return res.end("{}");
        }
        try {
          mkdirSync(PERF, { recursive: true });
          const report = JSON.parse(await body(req));
          const id = newId();
          // The screenshot goes beside the numbers as its own picture.
          const shot = typeof report.shot === "string" ? report.shot : "";
          delete report.shot;
          if (shot.startsWith("data:image/jpeg;base64,")) {
            writeFileSync(join(PERF, `${id}.jpg`), Buffer.from(shot.split(",")[1], "base64"));
            report.shot = `${id}.jpg`;
          }
          writeFileSync(join(PERF, `${id}.json`), `${JSON.stringify(report, null, 2)}\n`);
          res.end(JSON.stringify({ id }));
        } catch (e) {
          res.statusCode = 500;
          res.end(JSON.stringify({ error: String(e) }));
        }
      });
      server.middlewares.use("/__notes", async (req, res) => {
        mkdirSync(NOTES, { recursive: true });
        const send = (code: number, value: unknown) => {
          res.statusCode = code;
          res.setHeader("content-type", "application/json");
          res.end(JSON.stringify(value));
        };
        if (req.method !== "GET" && !sameOrigin(req)) return send(403, { error: "other origin" });
        try {
          const id = (req.url ?? "/").replace(/^\//, "").split("?")[0];
          if (req.method === "GET") {
            const notes = readdirSync(NOTES)
              .filter((f) => f.endsWith(".json"))
              .sort()
              .map((f) => JSON.parse(readFileSync(join(NOTES, f), "utf8")));
            return send(200, notes);
          }
          if (req.method === "POST") {
            const note = JSON.parse(await body(req));
            if (!note.id || !ID.test(note.id)) note.id = newId();
            note.created ??= new Date().toISOString();
            note.status ??= "open";
            note.replies ??= [];
            writeFileSync(join(NOTES, `${note.id}.json`), `${JSON.stringify(note, null, 2)}\n`);
            return send(200, note);
          }
          if (req.method === "DELETE" && ID.test(id)) {
            unlinkSync(join(NOTES, `${id}.json`));
            return send(200, { deleted: id });
          }
          send(400, { error: "bad request" });
        } catch (e) {
          send(500, { error: String(e) });
        }
      });
    },
  };
}

// Nunito is built into the wasm, so its license goes out beside it.
function fontLicense(): Plugin {
  const ofl = join(dirname(fileURLToPath(import.meta.url)), "../zawnabout-game/assets/nunito-OFL.txt");
  return {
    name: "zawnabout-font-license",
    apply: "build",
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "nunito-OFL.txt", source: readFileSync(ofl, "utf8") });
    },
  };
}

// One static page: index.html, a script bundle and the wasm file. `--mode lan` serves
// HTTPS with the self-signed certificate from scripts/lan-cert.mjs, for phones on the LAN.
export default defineConfig(({ mode }) => {
  const cert = join(dirname(fileURLToPath(import.meta.url)), ".cert");
  const https =
    mode === "lan"
      ? { key: readFileSync(join(cert, "key.pem")), cert: readFileSync(join(cert, "cert.pem")) }
      : undefined;
  return {
    base: "./",
    server: { port: mode === "lan" ? 5443 : 5180, https },
    build: { target: "es2024", assetsInlineLimit: 0 },
    plugins: [notesPlugin(), fontLicense()],
  };
});
