# DeckForge website

Live at **https://deckforge.adoxcol.com**.

A static page with no build step: `index.html`, `404.html`, `styles.css`, `main.js` and `assets/`.

## Hosting

Vercel deploys this folder from the repository root. `vercel.json` sets an empty build command and
`site` as the output directory, so Vercel serves these files as they are and never installs or
builds the desktop app. Pushes to `main` go to production at deckforge.adoxcol.com; other branches
and pull requests get preview URLs.

To set it up from scratch:

1. In Vercel, choose **Add New → Project**, import `Adoxcol/deckforge`, and deploy. Leave the
   settings as they are, because `vercel.json` overrides them.
2. Under **Settings → Domains**, add `deckforge.adoxcol.com`.
3. Where the DNS for `adoxcol.com` is managed, add the record Vercel shows for it. For a subdomain
   this is a `CNAME` named `deckforge`. If that DNS is on Cloudflare, set the record to
   **DNS only** (grey cloud) so Vercel can issue the certificate.

## Preview locally

```sh
python -m http.server 8080 --directory site
```

Then open http://localhost:8080.

## Things that update themselves

- **Download buttons** link to the releases page. On load, `main.js` asks the GitHub API for the
  latest release and points them straight at the `*-setup.exe` installer, with its version and size.
  If the API is unreachable or rate-limited, the releases-page link stays.

## Things to update by hand

- **Editor screenshots** (`assets/screens/`): run `npm run dev`, then
  `node scripts/capture-site-screens.mjs`. It needs Playwright with Chromium.
- **Hardware section**: keep it in line with the release notes. Move a device to “Tested on hardware”
  only after it has been checked on a real deck.
- **Demo deck** in the hero: its action names come from `src/data/actionCatalog.ts` and
  `src/data/extraActions.ts`, and its tap/double-press/hold timings from `src-tauri/src/gestures.rs`.
- **Social preview** (`assets/og.png`, 1200×630) and the `og:`/canonical URLs in `index.html`,
  which point at `https://deckforge.adoxcol.com/`.
