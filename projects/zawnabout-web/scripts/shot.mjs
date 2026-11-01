// Drives the real page in headless Chromium and takes screenshots, for checking the
// browser build without a person at the keyboard.
//
// node scripts/shot.mjs SCRIPT OUT_DIR [--size WxH] [--touch] [--swiftshader]
//
// SCRIPT lines run in real time:
//   wait SECONDS        let time pass
//   shot NAME.png       screenshot the page
//   key-down CODE       real keyboard event, such as Space or KeyW
//   key-up CODE
//   click X Y           real mouse click at page pixels
//   eval JS             evaluate in the page
//   anything else       a game command, such as `spawn 0 60 0 0 20`
// Run `pnpm wasm` first so pkg/ is current.
import { mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const opt = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const [scriptPath, outDir] = args.filter((a, i) => !a.startsWith("--") && !args[i - 1]?.startsWith("--size"));
if (!scriptPath || !outDir) {
  console.error("usage: node scripts/shot.mjs SCRIPT OUT_DIR [--size WxH] [--touch] [--swiftshader]");
  process.exit(2);
}
const [width, height] = opt("--size", "1280x720").split("x").map(Number);
mkdirSync(outDir, { recursive: true });

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const server = await createServer({ root, logLevel: "warn", server: { port: 5181, strictPort: false } });
await server.listen();
const url = server.resolvedUrls.local[0];

const browserArgs = ["--enable-unsafe-webgpu"];
if (flag("--swiftshader")) browserArgs.push("--use-webgpu-adapter=swiftshader");
// Full Chromium in new headless mode can reach the real GPU, the headless shell cannot.
const browser = await chromium.launch({ channel: "chromium", args: browserArgs });
const page = await browser.newPage({ viewport: { width, height }, hasTouch: flag("--touch") });
page.on("console", (m) => console.log(`[page] ${m.text()}`));
page.on("pageerror", (e) => console.log(`[page error] ${e.message}`));

const query = `?nowelcome&debug${flag("--touch") ? "&touch" : ""}`;
await page.goto(url + query);
await page.waitForFunction(() => window.zawnabout?.ready, null, { timeout: 30000 });

for (const raw of readFileSync(scriptPath, "utf8").split("\n")) {
  const line = raw.split("#")[0].trim();
  if (!line) continue;
  const [cmd, ...rest] = line.split(/\s+/);
  if (cmd === "wait") await page.waitForTimeout(Number(rest[0]) * 1000);
  else if (cmd === "shot") {
    const path = join(outDir, rest[0] ?? "shot.png");
    await page.screenshot({ path });
    console.log(`shot ${path}`);
  } else if (cmd === "key-down") await page.keyboard.down(rest[0]);
  else if (cmd === "key-up") await page.keyboard.up(rest[0]);
  else if (cmd === "click") await page.mouse.click(Number(rest[0]), Number(rest[1]));
  else if (cmd === "eval") console.log(await page.evaluate(rest.join(" ")));
  else {
    const out = await page.evaluate((l) => window.zawnabout.cmd(l), line);
    if (out) console.log(out);
  }
}

await browser.close();
await server.close();
