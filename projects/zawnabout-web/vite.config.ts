import { defineConfig } from "vite";

// One static page: index.html, a script bundle and the wasm file.
export default defineConfig({
  base: "./",
  server: { port: 5180 },
  build: { target: "es2024", assetsInlineLimit: 0 },
});
