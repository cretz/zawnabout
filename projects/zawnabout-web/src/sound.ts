import type { App } from "../pkg/zawnabout_web.js";

/** Plays the world's sound, made in Rust, through the page. Browsers only allow sound
 *  after the player does something, so it starts on the first click or key. */
export function setupSound(app: App) {
  let ctx: AudioContext | null = null;
  const start = () => {
    if (ctx) return;
    ctx = new AudioContext();
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
    app.set_sound(true);
  };
  addEventListener("pointerdown", start, { once: true, capture: true });
  addEventListener("keydown", start, { once: true, capture: true });
  // Quiet while the page is hidden.
  document.addEventListener("visibilitychange", () => {
    if (!ctx) return;
    if (document.hidden) ctx.suspend();
    else ctx.resume();
  });
}
