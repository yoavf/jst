# jst — Base44 dev notes

## What this is
A Cargo workspace (`crates/cli`, `crates/server`, `crates/shared`) plus a static
Vite-served website in `docs/` (marketing page + in-browser WASM shell demo).
The browser demo proxies its `/api/jst-demo` and `/api/jst-status` calls to the
already-hosted production server at `https://jst-server.fly.dev`, so the
frontend renders and is explorable without running the Rust server or any
external credentials.

## Running in the Base44 sandbox
`docker-compose.base44.yml` runs a single `node:22` service that bind-mounts
the repo, runs `npm install`, and starts the Vite dev server serving `docs/`
on port 3000.

- `vite.config.js` dev server is configured with `host: true`, `port: 3000`,
  and `allowedHosts: true` so the preview proxy hostname is accepted.
- `npm ci` fails because `package-lock.json` is out of sync; use `npm install`.
- Prebuilt demo bundles already live in `docs/assets/` (committed), so no
  `npm run build:demo` is needed to view the site.

## Verification
- `curl -sf -H "Host: external-preview.example.com" http://localhost:3000/`
  returns the page (HTTP 200) with `<title>jst — shell commands from plain English</title>`.
- `script.js`, `styles.css`, and the `/assets/*.js` demo bundles are served.

## When credentials are needed
The Rust server (`crates/server`) needs `LLM_API_URL` / `LLM_API_KEY` /
`LLM_MODEL` (and optionally `DEMO_LLM_API_KEY` for the browser demo endpoint)
only if you self-host the proxy. The frontend sandbox does **not** require any
secrets in this dev setup because it talks to the hosted fly.dev server.
