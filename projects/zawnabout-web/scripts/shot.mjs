// Drives the real page in headless Chromium and takes screenshots, for checking the
// browser build without a person at the keyboard.
//
// node scripts/shot.mjs SCRIPT OUT_DIR [--size WxH] [--dpr N] [--cpu N] [--touch] [--swiftshader] [--perf] [--load]
//
// --cpu N slows the page's main thread N times, to stand in for a slower device.
// --load saves a CPU profile of the page starting, as OUT_DIR/load.cpuprofile, and a
// screenshot of the loading screen as OUT_DIR/loading.png.
//
// SCRIPT lines run in real time:
//   wait SECONDS        let time pass
//   shot NAME.png       screenshot the page
//   key-down CODE       real keyboard event, such as Space or KeyW
//   key-up CODE
//   click X Y           real mouse click at page pixels
//   eval JS             evaluate in the page
//   frames-start        start timing frames, and each part of them
//   cpu-start           start Chrome's CPU profiler
//   cpu-stop NAME       save the profile as OUT_DIR/NAME.cpuprofile
//   frames NAME [MOST]  print frame gaps and the slowest parts since frames-start, then
//                       start over. With MOST, the run fails if more than that many
//                       frames took over 33 ms.
//   anything else       a game command, such as `spawn 0 60 0 0 20`
// Run `pnpm wasm` first so pkg/ is current.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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
const valued = ["--size", "--dpr", "--cpu"];
const [scriptPath, outDir] = args.filter((a, i) => !a.startsWith("--") && !valued.includes(args[i - 1]));
if (!scriptPath || !outDir) {
  console.error(
    "usage: node scripts/shot.mjs SCRIPT OUT_DIR [--size WxH] [--dpr N] [--cpu N] [--touch] [--swiftshader] [--perf] [--load]",
  );
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
const page = await browser.newPage({
  viewport: { width, height },
  deviceScaleFactor: Number(opt("--dpr", "1")),
  hasTouch: flag("--touch"),
});
const cdp = await page.context().newCDPSession(page);
if (opt("--cpu")) await cdp.send("Emulation.setCPUThrottlingRate", { rate: Number(opt("--cpu")) });
page.on("console", (m) => console.log(`[page] ${m.text()}`));
page.on("pageerror", (e) => console.log(`[page error] ${e.message}`));

const query = `?nowelcome&debug${flag("--touch") ? "&touch" : ""}${flag("--perf") ? "&perf" : ""}`;
if (flag("--load")) {
  await cdp.send("Profiler.enable");
  await cdp.send("Profiler.setSamplingInterval", { interval: 200 });
  await cdp.send("Profiler.start");
}
const opened = Date.now();
await page.goto(url + query);
if (flag("--load")) await page.screenshot({ path: join(outDir, "loading.png") });
await page.waitForFunction(() => window.zawnabout?.ready, null, { timeout: 30000 });
console.log(`ready after ${Date.now() - opened} ms`);
if (flag("--load")) {
  const { profile } = await cdp.send("Profiler.stop");
  writeFileSync(join(outDir, "load.cpuprofile"), JSON.stringify(profile));
}

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
  else if (cmd === "frames-start") await page.evaluate(framesStart);
  else if (cmd === "cpu-start") {
    await cdp.send("Profiler.enable");
    await cdp.send("Profiler.setSamplingInterval", { interval: 200 });
    await cdp.send("Profiler.start");
  } else if (cmd === "cpu-stop") {
    const { profile } = await cdp.send("Profiler.stop");
    const path = join(outDir, `${rest[0] ?? "cpu"}.cpuprofile`);
    writeFileSync(path, JSON.stringify(profile));
    console.log(`cpu profile ${path}`);
  }
  else if (cmd === "frames") {
    const { text, over } = await page.evaluate(framesReport);
    console.log(`frames ${rest[0] ?? ""} ${text}`);
    if (rest[1] !== undefined && over > Number(rest[1])) {
      console.log(`too slow: ${rest[0]} had ${over} frames over 33 ms, at most ${rest[1]} allowed`);
      process.exitCode = 1;
    }
  }
  else {
    const out = await page.evaluate((l) => window.zawnabout.cmd(l), line);
    if (out) console.log(out);
  }
}

await browser.close();
await server.close();

/** In the page: note the time between frames from here on, and time each part. */
function framesStart() {
  const w = window;
  w.zawnabout.app.perf_start();
  w.zawnabout.app.perf_report();
  w.frameGaps = [];
  w.frameLong = [];
  if (w.frameWatch) return;
  w.frameWatch = true;
  // How long the game's own frame call took, to tell its time from the page's and the
  // browser's in a long gap.
  const app = w.zawnabout.app;
  const frame = app.frame.bind(app);
  let inGame = 0;
  app.frame = (...a) => {
    const t = performance.now();
    const out = frame(...a);
    inGame = performance.now() - t;
    return out;
  };
  let last = performance.now();
  const tick = (now) => {
    const gap = now - last;
    w.frameGaps.push(gap);
    if (gap > 50) w.frameLong.push(`${gap.toFixed(0)} (game ${inGame.toFixed(0)})`);
    last = now;
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

/** In the page: the frame gaps and slowest parts since the last start, then start over. */
function framesReport() {
  const w = window;
  const gaps = [...w.frameGaps].sort((a, b) => a - b);
  w.frameGaps = [];
  const long = w.frameLong.splice(0).join(", ");
  const at = (q) => gaps[Math.min(gaps.length - 1, Math.floor(q * gaps.length))] ?? 0;
  const over = (ms) => gaps.filter((g) => g > ms).length;
  const total = gaps.reduce((a, b) => a + b, 0);
  const [parts, slowest] = w.zawnabout.app.perf_report().split("\n");
  const text =
    `${gaps.length} frames, ${((gaps.length * 1000) / Math.max(total, 1)).toFixed(1)} fps, ` +
    `median ${at(0.5).toFixed(1)} p95 ${at(0.95).toFixed(1)} max ${at(1).toFixed(1)} ms, ` +
    `over 33 ms ${over(33)}, over 50 ms ${over(50)}${long ? ` [${long}]` : ""}\n` +
    `  ${parts.split("; ").slice(0, 8).join("; ")}\n${slowest}`;
  return { text, over: over(33) };
}
