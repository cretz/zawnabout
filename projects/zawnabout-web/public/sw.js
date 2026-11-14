// Keeps a copy of the game so it opens with no internet. Online first: each launch asks
// the site for the page, and only when there is no answer in a few seconds does it use
// the copy, marked so the game can say so. Built files have their contents in their
// names, so a copy of one is never stale, and those the newest page no longer uses go.

const CACHE = "zawnabout";
const WAIT_MS = 3000;
/** Files with the same name in every build, kept fresh with the page. */
const STEADY = ["manifest.webmanifest", "icon-180.png", "icon-192.png", "icon-512.png"];
const PAGE = new URL("./", self.location.href).href;
const BUILT = /[\w.-]+-[\w-]{8}\.(?:js|css|wasm)/g;

self.addEventListener("install", (e) => {
  e.waitUntil(
    fresh()
      .then((f) => f.kept)
      .catch(() => {}),
  );
  self.skipWaiting();
});

self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (e) => {
  const req = e.request;
  const url = new URL(req.url);
  if (req.method !== "GET" || url.origin !== self.location.origin) return;
  if (req.mode === "navigate") e.respondWith(page(e));
  else if (url.pathname.match(BUILT)) e.respondWith(get(req.url));
  else e.respondWith(other(req));
});

/**
 * The newest page from the site, and once every built file it uses is kept, the page is
 * kept too, so a copy of the page always has its files.
 */
async function fresh() {
  const res = await fetch(PAGE, { cache: "no-cache" });
  if (!res.ok) throw new Error(`page ${res.status}`);
  const html = await res.clone().text();
  return { html, kept: keep(html, res) };
}

async function keep(html, res) {
  const cache = await caches.open(CACHE);
  const used = new Set();
  const todo = names(html, PAGE);
  while (todo.length) {
    const url = todo.pop();
    if (used.has(url)) continue;
    used.add(url);
    const file = await get(url);
    // A name that only looked like a built file.
    if (!file.ok) continue;
    if (url.endsWith(".js")) todo.push(...names(await file.text(), url));
  }
  for (const name of STEADY) {
    const url = new URL(name, PAGE).href;
    const file = await fetch(url, { cache: "no-cache" }).catch(() => null);
    if (file?.ok) await cache.put(url, file);
  }
  await cache.put(PAGE, res);
  for (const req of await cache.keys()) {
    if (new URL(req.url).pathname.match(BUILT) && !used.has(req.url)) await cache.delete(req);
  }
}

/** The built files a page or script names, as full addresses. */
function names(text, base) {
  return [...new Set(text.match(BUILT) ?? [])].map((n) => {
    const at = text.indexOf(n);
    const dir = text.slice(Math.max(0, at - 8), at).endsWith("assets/") ? "assets/" : "";
    return new URL(dir + n, base).href;
  });
}

/** The page: the site's if it answers in time, else the copy, marked as one. */
async function page(e) {
  const online = fresh();
  e.waitUntil(online.then((f) => f.kept).catch(() => {}));
  const late = new Promise((ok) => setTimeout(ok, WAIT_MS, null));
  try {
    const got = await Promise.race([online, late]);
    if (got) return html(got.html);
  } catch {
    // No internet, so the copy below.
  }
  // The first time there is no copy yet, so it waits for the site after all.
  const copy = await caches.match(PAGE);
  if (!copy) return html((await online).html);
  return html((await copy.text()).replace("<html", "<html data-offline"));
}

const html = (text) =>
  new Response(text, { headers: { "content-type": "text/html; charset=utf-8" } });

/** Downloads under way, so the page and the keeping share one of each. */
const loading = new Map();

/** A built file from the copies, else from the site, kept once it comes. */
async function get(url) {
  const copy = await caches.match(url);
  if (copy) return copy;
  if (!loading.has(url)) {
    const load = fetch(url).then(async (res) => {
      if (res.ok) await (await caches.open(CACHE)).put(url, res.clone());
      return res;
    });
    loading.set(url, load);
    load.finally(() => loading.delete(url)).catch(() => {});
  }
  return (await loading.get(url)).clone();
}

/** Anything else, such as the icons: the site's, else any copy. */
async function other(req) {
  try {
    const res = await fetch(req);
    if (res.ok) (await caches.open(CACHE)).put(req.url, res.clone());
    return res;
  } catch {
    return (await caches.match(req.url)) ?? Response.error();
  }
}
