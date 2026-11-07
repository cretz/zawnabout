# zawnabout-web

The browser host: a wasm build of the game plus a TypeScript page with touch controls, the minimap and notes mode. The journal and pause screen are drawn by the game. The page keeps settings, keys and the saved game in localStorage and handles save files. `?play=0` leaves gameplay off.

- `pnpm dev` serves it at http://localhost:5180, and `pnpm dev:lan` serves it over HTTPS on the local network for phones and tablets.
- `pnpm build` makes a static build in `dist/`.
- `node scripts/shot.mjs SCRIPT OUT` takes screenshots in headless Chromium.
