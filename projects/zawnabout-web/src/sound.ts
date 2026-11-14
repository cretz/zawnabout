import type { App } from "../pkg/zawnabout_web.js";

/** Plays the world's sound, made in Rust, through the page. Browsers only allow sound
 *  after the player does something, so it starts on the first click, tap or key. */
export function setupSound(app: App) {
  // Plays like a video does, so an iPad's silent switch does not hush it.
  const session = (navigator as Navigator & { audioSession?: { type: string } }).audioSession;
  if (session) session.type = "playback";
  // Made now, while the page is loading, since making it holds the page a moment. It
  // stays silent until the player does something.
  const ctx = new AudioContext();
  const node = ctx.createScriptProcessor(4096, 0, 2);
  let buf = new Float32Array(0);
  node.onaudioprocess = (e) => {
    const out = e.outputBuffer;
    const n = out.length;
    if (buf.length !== n * 2) buf = new Float32Array(n * 2);
    app.sound_fill(buf, out.sampleRate);
    const l = out.getChannelData(0);
    const r = out.getChannelData(1);
    for (let i = 0; i < n; i++) {
      l[i] = buf[i * 2];
      r[i] = buf[i * 2 + 1];
    }
  };
  node.connect(ctx.destination);
  let started = false;
  const start = () => {
    // iPads may make the sound wait until a tap lifts, so ask again on each one until
    // it plays.
    if (ctx.state !== "running" && !document.hidden) void ctx.resume();
    if (!started) {
      started = true;
      app.set_sound(true);
    }
  };
  // Quiet while the page is hidden.
  document.addEventListener("visibilitychange", () => {
    if (!started) return;
    if (document.hidden) void ctx.suspend();
    else void ctx.resume();
  });
}
