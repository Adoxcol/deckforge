import React from "react";
import { deckforgeApi } from "../bridge/tauriBridge";
import { createAppIconLoader } from "../lib/appIconCache";

const loadAppIcon = createAppIconLoader((exe) => deckforgeApi.appIcon(exe));

/** Process stem to a Simple Icons slug, for the browser preview and apps with no extractable icon. */
const BRAND_SLUG: Record<string, string> = {
  chrome: "googlechrome",
  discord: "discord",
  spotify: "spotify",
  obsidian: "obsidian",
  figma: "figma",
  notion: "notion",
  steam: "steam",
  steamwebhelper: "steam",
  dropbox: "dropbox",
  epicgameslauncher: "epicgames",
  riotclientservices: "riotgames",
  docker: "docker",
  "docker desktop": "docker",
  qbittorrent: "qbittorrent",
  nvidia: "nvidia",
  "nvidia share": "nvidia",
};

type Brand = { hex: string; path: string };

let brands: Promise<Map<string, Brand>> | null = null;

function brandMap(): Promise<Map<string, Brand>> {
  brands ??= import("virtual:brand-icons")
    .then((mod) => {
      const map = new Map<string, Brand>();
      for (const icon of mod.default) map.set(icon.slug, { hex: icon.hex, path: icon.path });
      return map;
    })
    .catch(() => new Map());
  return brands;
}

const stemOf = (name: string, path?: string) => {
  const file = path?.split(/[\\/]/).pop()?.trim().toLowerCase() ?? "";
  const raw = (file || name).trim().toLowerCase().replace(/\.exe$/i, "");
  return raw;
};

/** The name the Windows icon lookup expects. */
const exeOf = (stem: string) => (stem ? (stem.endsWith(".exe") ? stem : `${stem}.exe`) : "");

/** A program's own icon, then its brand mark, then a letter. */
export const AppMark: React.FC<{ name: string; path?: string; size?: number }> = ({ name, path, size = 22 }) => {
  const stem = stemOf(name, path);
  const exe = exeOf(stem);
  const slug = BRAND_SLUG[stem];
  const [src, setSrc] = React.useState<{ exe: string; url: string | null } | null>(null);
  const [brand, setBrand] = React.useState<{ slug: string; icon: Brand | null } | null>(null);

  React.useEffect(() => {
    let alive = true;
    if (!exe) return;
    loadAppIcon(exe).then((url) => {
      if (alive) setSrc({ exe, url });
    }).catch(() => {});
    return () => {
      alive = false;
    };
  }, [exe]);

  React.useEffect(() => {
    if (!slug) return;
    let alive = true;
    brandMap().then((map) => {
      if (alive) setBrand({ slug, icon: map.get(slug) ?? null });
    });
    return () => {
      alive = false;
    };
  }, [slug]);

  const shownSrc = src?.exe === exe ? src.url : null;
  const shownBrand = brand?.slug === slug ? brand.icon : null;
  const letter = (stem || name || "?").charAt(0).toUpperCase();
  const hue = [...stem].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);

  return (
    <span
      className="shrink-0 rounded-[5px] flex items-center justify-center overflow-hidden"
      style={{
        width: size,
        height: size,
        background: shownSrc || shownBrand ? "var(--bg-elevated)" : `hsl(${hue} 42% 24%)`,
        color: `hsl(${hue} 75% 84%)`,
      }}
      aria-hidden
    >
      {shownSrc ? (
        <img src={shownSrc} alt="" className="w-full h-full object-contain" draggable={false} />
      ) : shownBrand ? (
        <svg viewBox="0 0 24 24" width={Math.round(size * 0.68)} height={Math.round(size * 0.68)} aria-hidden>
          <path d={shownBrand.path} fill={`#${shownBrand.hex}`} />
        </svg>
      ) : (
        <span className="font-semibold leading-none" style={{ fontSize: Math.max(10, Math.round(size * 0.48)) }}>{letter}</span>
      )}
    </span>
  );
};
