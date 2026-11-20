# DeckForge website

A static page with no build step: `index.html`, `styles.css`, `main.js` and `assets/`.
The `Website` workflow (`.github/workflows/pages.yml`) publishes this folder to GitHub Pages
whenever it changes on `main`.

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
  which assume `https://adoxcol.github.io/deckforge/`. Change them if the site moves to its own domain.
