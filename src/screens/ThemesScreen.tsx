import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ImagePlus, RefreshCw, Sparkles, Wand2 } from "lucide-react";
import type { KeyConfig } from "../types/deckforge";
import { DeckIcon } from "../components/DeckIcon";
import { KEY_THEMES, THEME_DELAY_MS, drawThemeFrame, renderThemeFrames, svgToImage, type ThemeId } from "../lib/keyThemes";

/** Frames for one key, ready to stage. Frames are base64 PNG without the data URL prefix. */
export interface KeyFrames {
  index: number;
  name: string;
  frames: string[];
  delayMs: number;
}

const MAX_WALLPAPER_FRAMES = 48;

async function iconImage(key: KeyConfig, color: string): Promise<HTMLImageElement | null> {
  if (!key.iconName) return null;
  const svg = renderToStaticMarkup(<DeckIcon name={key.iconName} size={96} color={color} />);
  return svgToImage(svg.includes("xmlns") ? svg : svg.replace("<svg", '<svg xmlns="http://www.w3.org/2000/svg"'));
}

/** A small live preview of a theme on three keys. */
const ThemePreview: React.FC<{ theme: ThemeId; icons: string[] }> = ({ theme, icons }) => {
  const refs = React.useRef<Array<HTMLCanvasElement | null>>([]);
  React.useEffect(() => {
    let alive = true;
    let frame = 0;
    let raf = 0;
    const color = KEY_THEMES.find((t) => t.id === theme)?.iconColor ?? "#FFFFFF";
    const images: Array<HTMLImageElement | null> = icons.map(() => null);
    const reducedMotion = typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const drawAll = (progress: number) => {
      refs.current.forEach((canvas, i) => {
        const ctx = canvas?.getContext("2d");
        if (ctx) drawThemeFrame(ctx, theme, progress, 112, i + 1, images[i]);
      });
    };
    icons.forEach((name, i) =>
      void iconImage({ iconName: name } as KeyConfig, color).then((img) => {
        images[i] = img;
        if (reducedMotion && alive) drawAll(0.25);
      })
    );
    if (reducedMotion) {
      drawAll(0.25);
      return () => {
        alive = false;
      };
    }
    const loop = () => {
      if (!alive) return;
      frame += 1;
      drawAll((frame / 60) % 1);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => {
      alive = false;
      cancelAnimationFrame(raf);
    };
  }, [theme, icons]);
  return (
    <div className="flex justify-center gap-2.5 p-2.5 rounded-xl bg-[var(--bg-app)] border border-[var(--border-subtle)] w-full">
      {icons.map((name, i) => (
        <canvas key={name} ref={(el) => { refs.current[i] = el; }} width={112} height={112} className="w-16 h-16 rounded-lg border border-[var(--border-strong)]/60 bg-[#0B0D14] shadow-md" />
      ))}
    </div>
  );
};

/** Decodes every frame of a GIF or still image, with its delay in milliseconds. */
async function decodeFrames(file: File): Promise<{ frames: ImageBitmap[]; delays: number[] }> {
  const Decoder = (window as unknown as { ImageDecoder?: new (init: { data: ArrayBuffer; type: string }) => {
    tracks: { ready: Promise<void>; selectedTrack?: { frameCount: number } };
    decode: (opt: { frameIndex: number }) => Promise<{ image: VideoFrame }>;
    close: () => void;
  } }).ImageDecoder;
  if (Decoder && file.type) {
    const decoder = new Decoder({ data: await file.arrayBuffer(), type: file.type });
    await decoder.tracks.ready;
    const count = decoder.tracks.selectedTrack?.frameCount ?? 1;
    const step = Math.max(1, Math.ceil(count / MAX_WALLPAPER_FRAMES));
    const frames: ImageBitmap[] = [];
    const delays: number[] = [];
    for (let i = 0; i < count; i += step) {
      const { image } = await decoder.decode({ frameIndex: i });
      const ms = ((image.duration ?? 100_000) / 1000) * step;
      frames.push(await createImageBitmap(image));
      delays.push(ms);
      image.close();
    }
    decoder.close();
    return { frames, delays };
  }
  return { frames: [await createImageBitmap(file)], delays: [100] };
}

export const ThemesScreen: React.FC<{
  keys: KeyConfig[];
  columns: number;
  rows: number;
  pixelWidth: number;
  pixelHeight: number;
  selectedIndex: number;
  onApplyFrames: (items: KeyFrames[], label: string) => Promise<void>;
  notify: (text: string, kind?: "success" | "info" | "warning") => void;
}> = ({ keys, columns, rows, pixelWidth, pixelHeight, selectedIndex, onApplyFrames, notify }) => {
  const [busy, setBusy] = React.useState<string | null>(null);
  const fileInput = React.useRef<HTMLInputElement>(null);
  const previewIcons = React.useMemo(() => ["Play", "Music", "Settings"], []);

  const applyTheme = async (theme: ThemeId, onlySelected: boolean) => {
    const def = KEY_THEMES.find((t) => t.id === theme)!;
    setBusy(`${theme}:${onlySelected}`);
    try {
      const size = Math.max(pixelWidth, pixelHeight, 72);
      const targets = onlySelected ? keys.filter((k) => k.index === selectedIndex) : keys;
      const items: KeyFrames[] = [];
      for (const key of targets) {
        const icon = await iconImage(key, def.iconColor).catch(() => null);
        items.push({ index: key.index, name: `${def.name}.gif`, frames: renderThemeFrames(theme, size, key.index + 1, icon), delayMs: THEME_DELAY_MS });
        await new Promise((r) => setTimeout(r));
      }
      await onApplyFrames(items, def.name);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error), "warning");
    } finally {
      setBusy(null);
    }
  };

  /** Splits one picture or GIF across the whole page, so the deck shows one big animation. */
  const applyWallpaper = async (file: File) => {
    setBusy("wallpaper");
    try {
      const { frames, delays } = await decodeFrames(file);
      // Keys have gaps between them; leave a matching gap in the picture so it lines up.
      const gap = Math.round(pixelWidth * 0.18);
      const fullW = columns * pixelWidth + (columns - 1) * gap;
      const fullH = rows * pixelHeight + (rows - 1) * gap;
      const canvas = document.createElement("canvas");
      canvas.width = pixelWidth;
      canvas.height = pixelHeight;
      const ctx = canvas.getContext("2d")!;
      const delay = Math.max(40, Math.round(delays.reduce((a, b) => a + b, 0) / delays.length));
      const items: KeyFrames[] = keys.map((key) => ({ index: key.index, name: file.name, frames: [], delayMs: delay }));
      for (const bmp of frames) {
        // Cover: scale the picture to fill the whole deck, cropping the overflow.
        const scale = Math.max(fullW / bmp.width, fullH / bmp.height);
        const dw = bmp.width * scale;
        const dh = bmp.height * scale;
        const ox = (fullW - dw) / 2;
        const oy = (fullH - dh) / 2;
        for (const item of items) {
          const col = item.index % columns;
          const row = Math.floor(item.index / columns);
          const kx = col * (pixelWidth + gap);
          const ky = row * (pixelHeight + gap);
          ctx.clearRect(0, 0, pixelWidth, pixelHeight);
          ctx.drawImage(bmp, ox - kx, oy - ky, dw, dh);
          item.frames.push(canvas.toDataURL("image/png").split(",")[1] ?? "");
        }
        bmp.close();
        await new Promise((r) => setTimeout(r));
      }
      await onApplyFrames(items, file.name);
    } catch (error) {
      notify(error instanceof Error ? error.message : "Could not read that picture", "warning");
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="flex-1 min-h-0 overflow-y-auto animate-screen-in">
    <div className="p-6 max-w-6xl mx-auto w-full flex flex-col gap-6">
      <div className="pb-4 border-b border-[var(--border-subtle)]">
        <h1 className="text-xl font-bold flex items-center gap-2"><Sparkles size={18} className="text-[var(--accent-text)]" /> Themes</h1>
        <p className="text-xs text-[var(--text-secondary)] mt-1">
          Animated looks for the current page. Each key keeps its own icon on top. Press Apply to Device afterwards to see them on the deck.
        </p>
      </div>

      <section className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] transition-colors flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="font-bold text-sm flex items-center gap-2"><Wand2 size={15} className="text-[var(--accent-text)]" /> Full-deck wallpaper</div>
          <p className="text-xs text-[var(--text-secondary)] mt-1 max-w-xl">
            Pick a GIF or picture and it is split across every key, so the whole deck plays one big animation, like the skins in the AJAZZ app.
            Long GIFs are trimmed to {MAX_WALLPAPER_FRAMES} frames.
          </p>
        </div>
        <input
          ref={fileInput}
          type="file"
          accept="image/gif,image/png,image/jpeg,image/webp"
          className="hidden"
          onChange={(e) => {
            const file = e.target.files?.[0];
            e.target.value = "";
            if (file) void applyWallpaper(file);
          }}
        />
        <button
          type="button"
          disabled={busy !== null}
          onClick={() => fileInput.current?.click()}
          className="h-9 px-4 shrink-0 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white flex items-center gap-1.5 cursor-pointer disabled:opacity-50 active:scale-[0.98] transition-all"
        >
          {busy === "wallpaper" ? <RefreshCw size={14} className="animate-spin" /> : <ImagePlus size={14} />}
          {busy === "wallpaper" ? "Splitting…" : "Choose GIF"}
        </button>
      </section>

      <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
        {KEY_THEMES.map((theme) => (
          <div key={theme.id} className="p-4 rounded-xl bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:-translate-y-[1px] hover:shadow-lg transition-all flex flex-col gap-3">
            <ThemePreview theme={theme.id} icons={previewIcons} />
            <div>
              <div className="font-bold text-sm">{theme.name}</div>
              <div className="text-xs text-[var(--text-secondary)] mt-0.5">{theme.description}</div>
            </div>
            <div className="flex gap-2 mt-auto">
              <button
                type="button"
                disabled={busy !== null}
                onClick={() => void applyTheme(theme.id, false)}
                className="flex-1 h-8 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-50 transition-colors"
              >
                {busy === `${theme.id}:false` && <RefreshCw size={12} className="animate-spin" />}
                {busy === `${theme.id}:false` ? "Drawing…" : "Apply to page"}
              </button>
              <button
                type="button"
                disabled={busy !== null}
                onClick={() => void applyTheme(theme.id, true)}
                className="h-8 px-3 rounded-md border border-[var(--border-subtle)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-50 transition-colors"
              >
                {busy === `${theme.id}:true` && <RefreshCw size={12} className="animate-spin" />}
                {busy === `${theme.id}:true` ? "Drawing…" : `Key ${selectedIndex + 1} only`}
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
    </div>
  );
};
