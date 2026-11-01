// Builds the wasm crate in release and generates JS bindings into pkg/.
// The wasm-bindgen CLI must match the crate version exactly, so a matching CLI is
// installed into the cargo target dir when the one on PATH differs.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const projects = join(here, "..", "..");
const run = (cmd, args, opts = {}) => execFileSync(cmd, args, { stdio: "inherit", cwd: projects, ...opts });

run("cargo", ["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "zawnabout-web"]);

const lock = readFileSync(join(projects, "Cargo.lock"), "utf8");
const version = lock.match(/name = "wasm-bindgen"\nversion = "([^"]+)"/)[1];
const toolRoot = join(projects, "target", "tools");
const exe = process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
let bindgen = "wasm-bindgen";
const onPath = (() => {
  try {
    return execFileSync(bindgen, ["--version"], { encoding: "utf8" }).trim().split(" ")[1];
  } catch {
    return null;
  }
})();
if (onPath !== version) {
  bindgen = join(toolRoot, "bin", exe);
  if (!existsSync(bindgen) || execFileSync(bindgen, ["--version"], { encoding: "utf8" }).trim().split(" ")[1] !== version) {
    run("cargo", ["install", "wasm-bindgen-cli", "--version", version, "--locked", "--root", toolRoot]);
  }
}
const wasm = join(projects, "target", "wasm32-unknown-unknown", "release", "zawnabout_web.wasm");
run(bindgen, ["--target", "web", "--out-dir", join(here, "..", "pkg"), wasm]);
