// Makes a self-signed certificate for this machine's LAN addresses, so phones and tablets
// on the same network can open the dev server over HTTPS (WebGPU needs a secure page).
// Kept in .cert/, which git ignores. Delete it to make a fresh one when addresses change.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import { networkInterfaces } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const dir = join(dirname(fileURLToPath(import.meta.url)), "..", ".cert");
const key = join(dir, "key.pem");
const cert = join(dir, "cert.pem");
if (existsSync(key) && existsSync(cert)) process.exit(0);
mkdirSync(dir, { recursive: true });

const ips = Object.values(networkInterfaces())
  .flat()
  .filter((a) => a && a.family === "IPv4")
  .map((a) => a.address);
const san = ["DNS:localhost", ...ips.map((ip) => `IP:${ip}`)].join(",");
execFileSync("openssl", [
  "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "825",
  "-keyout", key, "-out", cert, "-subj", "/CN=zawnabout dev", "-addext", `subjectAltName=${san}`,
]);
console.log(`made a dev certificate for ${san}`);
