// Captures the editor screens shown on the website (site/assets/screens).
//
// The editor runs in a plain browser with its built-in demo profile, so no deck is needed:
//   1. npm run dev                          (serves the editor on http://localhost:1420)
//   2. node scripts/capture-site-screens.mjs
//
// Needs Playwright with Chromium (`npm i -g playwright && npx playwright install chromium`).
// Each screen is written as <name>-720.webp, -1440.webp and -2880.webp, encoded by Chromium itself,
// so phones, desktops and high-density displays each get a size close to what they show.

import { createRequire } from "node:module";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
let chromium;
try {
  ({ chromium } = require("playwright"));
} catch {
  console.error("Playwright is not installed. Run: npm i -g playwright && npx playwright install chromium");
  process.exit(1);
}

const BASE_URL = process.env.DECKFORGE_URL ?? "http://localhost:1420/";
const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), "..", "site", "assets", "screens");
const VIEWPORT = { width: 1440, height: 900 };
const WIDTHS = [720, 1440, 2880];
const QUALITY = 0.86;

// `nav` is the sidebar button's accessible name; null stays on Profiles and keys.
const SCREENS = [
  { file: "workspace", nav: null },
  { file: "macro-studio", nav: "Macro studio" },
  { file: "game-mode", nav: "Game mode" },
  { file: "icon-studio", nav: "Icon studio", waitForGone: "Loading logos" },
  { file: "themes", nav: "Themes" },
  { file: "devices", nav: "Devices" },
];

try {
  await fetch(BASE_URL);
} catch {
  console.error(`Nothing is answering at ${BASE_URL}. Start the editor with \`npm run dev\` first.`);
  process.exit(1);
}

await mkdir(OUT_DIR, { recursive: true });
const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: 2 });
  const encoder = await browser.newPage();

  for (const screen of SCREENS) {
    await page.goto(BASE_URL, { waitUntil: "networkidle" });
    if (screen.nav) {
      await page.getByRole("button", { name: screen.nav, exact: true }).first().click();
    }
    if (screen.waitForGone) {
      await page.getByText(screen.waitForGone).waitFor({ state: "detached", timeout: 15000 }).catch(() => {});
    }
    // Keep sidebar tooltips and focus rings out of the picture.
    await page.mouse.move(VIEWPORT.width - 4, VIEWPORT.height / 2);
    await page.evaluate(() => document.activeElement instanceof HTMLElement && document.activeElement.blur());
    await page.waitForTimeout(900);

    const png = await page.screenshot({ type: "png" });
    for (const width of WIDTHS) {
      const dataUrl = await encoder.evaluate(
        async ({ src, width, quality }) => {
          const img = new Image();
          img.src = src;
          await img.decode();
          const canvas = document.createElement("canvas");
          canvas.width = width;
          canvas.height = Math.round((img.naturalHeight / img.naturalWidth) * width);
          const ctx = canvas.getContext("2d");
          ctx.imageSmoothingQuality = "high";
          ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
          return canvas.toDataURL("image/webp", quality);
        },
        { src: `data:image/png;base64,${png.toString("base64")}`, width, quality: QUALITY },
      );
      const target = join(OUT_DIR, `${screen.file}-${width}.webp`);
      await writeFile(target, Buffer.from(dataUrl.split(",")[1], "base64"));
      console.log(`wrote ${target}`);
    }
  }
} finally {
  await browser.close();
}
