import React from "react";
import { Search, X } from "lucide-react";
import type { SimpleIcon } from "simple-icons";

type BrandIcon = Pick<SimpleIcon, "title" | "slug" | "hex" | "path">;

/** Shown before the user types anything. */
const POPULAR = [
  "spotify", "youtube", "twitch", "discord", "x", "instagram", "facebook", "tiktok", "reddit", "whatsapp",
  "telegram", "snapchat", "pinterest", "threads", "bluesky", "mastodon", "kick", "steam", "epicgames", "battledotnet",
  "riotgames", "valorant", "leagueoflegends", "counterstrike", "playstation", "obsstudio", "streamlabs", "googlechrome",
  "firefoxbrowser", "brave", "github", "notion", "obsidian", "figma", "adobephotoshop", "adobepremierepro",
  "davinciresolve", "blender", "applemusic", "soundcloud", "netflix", "primevideo", "googlemeet", "zoom", "gmail",
  "googledrive", "dropbox", "elgato", "nvidia", "amd", "razer", "logitechg", "openai", "claude", "chatgpt", "vlcmediaplayer",
];

let loaded: Promise<BrandIcon[]> | null = null;

/** The compact catalog stays lazy, so normal startup does not load logo paths. */
function loadIcons(): Promise<BrandIcon[]> {
  loaded ??= import("virtual:brand-icons").then((mod) => mod.default).catch((error) => {
    loaded = null;
    throw error;
  });
  return loaded;
}

function isLight(hex: string): boolean {
  const n = parseInt(hex, 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  return 0.299 * r + 0.587 * g + 0.114 * b > 170;
}

type Look = "brand" | "dark";

/** Draws a logo as a square PNG key image. */
async function renderKeyImage(icon: BrandIcon, look: Look): Promise<File> {
  const size = 288;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Could not draw the icon");
  const brand = `#${icon.hex}`;
  ctx.fillStyle = look === "brand" ? brand : "#12151F";
  ctx.fillRect(0, 0, size, size);
  const glyph = size * 0.56;
  ctx.translate((size - glyph) / 2, (size - glyph) / 2);
  ctx.scale(glyph / 24, glyph / 24);
  // Brand look: white or black logo on the brand color. Dark look: the logo in its brand color,
  // or white when that color would vanish on a dark key.
  const darkBrand = parseInt(icon.hex, 16) < 0x333333;
  ctx.fillStyle = look === "brand" ? (isLight(icon.hex) ? "#111111" : "#FFFFFF") : darkBrand ? "#FFFFFF" : brand;
  ctx.fill(new Path2D(icon.path));
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (!blob) throw new Error("Could not draw the icon");
  return new File([blob], `${icon.slug}.png`, { type: "image/png" });
}

/** Search popular app and social media logos and put one on the key. */
export const BrandIconPicker: React.FC<{ onPick: (file: File, icon: BrandIcon) => void }> = ({ onPick }) => {
  const [icons, setIcons] = React.useState<BrandIcon[] | null>(null);
  const [query, setQuery] = React.useState("");
  const [look, setLook] = React.useState<Look>("brand");
  const [failed, setFailed] = React.useState(false);

  React.useEffect(() => {
    loadIcons().then(setIcons).catch(() => setFailed(true));
  }, []);

  const shown = React.useMemo(() => {
    if (!icons) return [];
    const q = query.trim().toLowerCase();
    if (!q) {
      const bySlug = new Map(icons.map((i) => [i.slug, i]));
      return POPULAR.map((s) => bySlug.get(s)).filter((i): i is BrandIcon => !!i);
    }
    const starts = icons.filter((i) => i.title.toLowerCase().startsWith(q));
    const contains = icons.filter((i) => !i.title.toLowerCase().startsWith(q) && i.title.toLowerCase().includes(q));
    return [...starts, ...contains].slice(0, 80);
  }, [icons, query]);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs text-[var(--text-secondary)]">App and brand logos</span>
        <div role="group" aria-label="Logo style" className="flex gap-0.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] p-0.5 rounded-md text-[10.5px]">
          {(["brand", "dark"] as const).map((l) => (
            <button
              key={l}
              type="button"
              onClick={() => setLook(l)}
              aria-pressed={look === l}
              className={`px-2 py-0.5 rounded cursor-pointer transition-colors ${
                look === l ? "bg-[var(--accent-soft)] text-white font-medium" : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
              }`}
            >
              {l === "brand" ? "Brand color" : "Dark"}
            </button>
          ))}
        </div>
      </div>
      <div className="flex items-center gap-2 h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus-within:border-[var(--accent-primary)] focus-within:ring-2 focus-within:ring-[var(--accent-primary)]/20 transition-all">
        <Search size={13} className="text-[var(--text-muted)] shrink-0" />
        <input
          aria-label="Search app and brand logos"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search 3,000+ logos (Spotify, Discord…)"
          className="flex-1 min-w-0 bg-transparent outline-none text-xs text-[var(--text-primary)] placeholder:text-[var(--text-muted)]"
        />
        {query && (
          <button
            type="button"
            aria-label="Clear logo search"
            onClick={() => setQuery("")}
            className="p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
          >
            <X size={12} />
          </button>
        )}
      </div>
      <div className="grid grid-cols-5 gap-1.5 max-h-48 overflow-y-auto p-1.5 bg-[var(--bg-input)] border border-[var(--border-subtle)] rounded-md">
        {failed && <span className="col-span-5 text-[11px] text-[var(--status-warning)] p-2">Could not load the logos.</span>}
        {!icons && !failed && <span className="col-span-5 text-[11px] text-[var(--text-muted)] p-2">Loading logos…</span>}
        {icons && shown.length === 0 && <span className="col-span-5 text-[11px] text-[var(--text-muted)] p-2">No logo found.</span>}
        {shown.map((icon) => (
          <button
            key={icon.slug}
            type="button"
            title={icon.title}
            aria-label={`Use ${icon.title} logo`}
            onClick={() => void renderKeyImage(icon, look).then((file) => onPick(file, icon))}
            className="h-9 rounded-md flex items-center justify-center cursor-pointer hover:bg-[var(--bg-elevated)] border border-transparent hover:border-[var(--border-subtle)] transition-colors"
          >
            <svg viewBox="0 0 24 24" width={16} height={16} fill={isLight(icon.hex) || icon.hex === "000000" ? "#E8EAF0" : `#${icon.hex}`} aria-hidden>
              <path d={icon.path} />
            </svg>
          </button>
        ))}
      </div>
      <p className="text-[10.5px] text-[var(--text-muted)] leading-snug">
        Picking a logo sets it as the key&apos;s image. Logos come from Simple Icons and belong to their owners.
      </p>
    </div>
  );
};
