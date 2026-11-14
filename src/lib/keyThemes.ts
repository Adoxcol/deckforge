/** Animated key backgrounds. Each theme draws one frame for a point in its loop (0 to 1),
 * so the animation repeats without a jump. */

export type ThemeId = "synth" | "matrix" | "aurora" | "ember" | "glitch" | "warp";

export interface KeyTheme {
  id: ThemeId;
  name: string;
  description: string;
  /** Color of the key's icon drawn on top. */
  iconColor: string;
}

export const KEY_THEMES: KeyTheme[] = [
  { id: "synth", name: "Synthwave", description: "Neon sunset with a moving grid", iconColor: "#FFFFFF" },
  { id: "matrix", name: "Code rain", description: "Green characters falling down", iconColor: "#D1FAE5" },
  { id: "aurora", name: "Aurora", description: "Slow flowing bands of color", iconColor: "#FFFFFF" },
  { id: "ember", name: "Ember", description: "Sparks rising over charcoal", iconColor: "#FED7AA" },
  { id: "glitch", name: "Glitch", description: "Color split with scanlines", iconColor: "#FFFFFF" },
  { id: "warp", name: "Warp", description: "Stars flying past", iconColor: "#C7D2FE" },
];

export const THEME_FRAMES = 24;
export const THEME_DELAY_MS = 70;

/** Small repeatable random numbers, so each key gets its own but stable pattern. */
function rand(seed: number): () => number {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return (s >>> 0) / 4294967296;
  };
}

const TAU = Math.PI * 2;

/** Draws the background of `theme` at loop point `t` (0..1) on a square of `size` pixels. */
export function drawThemeBackground(ctx: CanvasRenderingContext2D, theme: ThemeId, t: number, size: number, seed = 1) {
  const S = size;
  const u = S / 112;
  const r = rand(seed * 7919 + 13);
  ctx.save();
  ctx.globalAlpha = 1;
  switch (theme) {
    case "synth": {
      ctx.fillStyle = "#1A0B2E";
      ctx.fillRect(0, 0, S, S);
      ctx.fillStyle = "#FF6AC1";
      ctx.beginPath();
      ctx.arc(S / 2, S * 0.52, S * 0.24, Math.PI, 0);
      ctx.fill();
      ctx.fillStyle = "#1A0B2E";
      for (let i = 0; i < 4; i++) ctx.fillRect(S * 0.26, S * 0.41 + i * 4 * u, S * 0.48, 1.5 * u);
      ctx.strokeStyle = "#22D3EE";
      ctx.lineWidth = u;
      const horizon = S * 0.54;
      for (let i = -6; i <= 6; i++) {
        ctx.beginPath();
        ctx.moveTo(S / 2 + i * 4 * u, horizon);
        ctx.lineTo(S / 2 + i * 22 * u, S);
        ctx.stroke();
      }
      const gap = (S - horizon) / 5;
      for (let i = 0; i < 6; i++) {
        const y = horizon + ((i + t) % 6) * gap;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(S, y);
        ctx.stroke();
      }
      break;
    }
    case "matrix": {
      ctx.fillStyle = "#020802";
      ctx.fillRect(0, 0, S, S);
      ctx.font = `${11 * u}px monospace`;
      const cols = 10;
      for (let c = 0; c < cols; c++) {
        const offset = r();
        const speed = 1 + Math.floor(r() * 2);
        const head = ((t * speed + offset) % 1) * (S + 60 * u) - 20 * u;
        for (let k = 0; k < 7; k++) {
          const y = head - k * 12 * u;
          if (y < -12 * u || y > S + 12 * u) continue;
          ctx.fillStyle = k === 0 ? "#BBF7D0" : "#22C55E";
          ctx.globalAlpha = 1 - k / 7;
          ctx.fillText(String.fromCharCode(0x30a0 + Math.floor(r() * 90)), c * 11 * u + 2 * u, y);
        }
      }
      break;
    }
    case "aurora": {
      ctx.fillStyle = "#06121F";
      ctx.fillRect(0, 0, S, S);
      ["#22D3EE", "#A78BFA", "#34D399"].forEach((color, i) => {
        ctx.fillStyle = color;
        ctx.globalAlpha = 0.35;
        ctx.beginPath();
        ctx.moveTo(0, S);
        for (let px = 0; px <= S; px += 2 * u) {
          ctx.lineTo(px, S * 0.36 + i * 14 * u + Math.sin(px / (18 * u) + t * TAU + i + seed) * 12 * u);
        }
        ctx.lineTo(S, S);
        ctx.fill();
      });
      break;
    }
    case "ember": {
      ctx.fillStyle = "#140C0A";
      ctx.fillRect(0, 0, S, S);
      for (let i = 0; i < 18; i++) {
        const x0 = r() * S;
        const phase = r();
        const fast = r() > 0.5;
        const p = (t * (fast ? 2 : 1) + phase) % 1;
        const y = S - p * S;
        const x = x0 + Math.sin(p * TAU * 2 + i) * 3 * u;
        ctx.fillStyle = fast ? "#FBBF24" : "#F97316";
        ctx.globalAlpha = 1 - p;
        ctx.fillRect(x, y, 2.5 * u, 2.5 * u);
      }
      break;
    }
    case "glitch": {
      ctx.fillStyle = "#0B0B12";
      ctx.fillRect(0, 0, S, S);
      ctx.fillStyle = "rgba(255,255,255,0.06)";
      const shift = Math.floor(t * THEME_FRAMES) % 4;
      for (let y = shift * u; y < S; y += 4 * u) ctx.fillRect(0, y, S, u);
      break;
    }
    case "warp": {
      ctx.fillStyle = "#03040A";
      ctx.fillRect(0, 0, S, S);
      for (let i = 0; i < 40; i++) {
        const angle = r() * TAU;
        const phase = r();
        const p = (t + phase) % 1;
        const dist = p * p * S * 0.75;
        const dot = (0.6 + p * 2) * u;
        ctx.fillStyle = "#E0E7FF";
        ctx.globalAlpha = Math.min(1, p * 2);
        ctx.fillRect(S / 2 + Math.cos(angle) * dist, S / 2 + Math.sin(angle) * dist, dot, dot);
      }
      break;
    }
  }
  ctx.restore();
}

/** Draws the icon image over a frame. Glitch splits it into red and blue copies. */
function drawIcon(ctx: CanvasRenderingContext2D, theme: ThemeId, t: number, size: number, icon: CanvasImageSource | null) {
  if (!icon) return;
  const box = size * 0.46;
  const x = (size - box) / 2;
  const y = (size - box) / 2;
  if (theme === "glitch") {
    const frame = Math.floor(t * THEME_FRAMES);
    const jump = frame % 12 < 2 ? size * 0.05 : size * 0.012;
    ctx.save();
    ctx.globalCompositeOperation = "lighter";
    ctx.globalAlpha = 0.9;
    ctx.drawImage(tint(icon, size, "#FF0050"), x - jump, y);
    ctx.drawImage(tint(icon, size, "#00E5FF"), x + jump, y);
    ctx.restore();
    if (frame % 12 < 2) {
      ctx.fillStyle = "#0B0B12";
      ctx.fillRect(0, size * 0.42 + (frame % 3) * size * 0.06, size, size * 0.035);
    }
    return;
  }
  ctx.drawImage(icon, x, y, box, box);
}

const tintCache = new Map<string, HTMLCanvasElement>();
let tintSource: CanvasImageSource | null = null;
function tint(icon: CanvasImageSource, size: number, color: string): HTMLCanvasElement {
  if (tintSource !== icon) {
    tintCache.clear();
    tintSource = icon;
  }
  const key = `${size}:${color}`;
  const hit = tintCache.get(key);
  if (hit) return hit;
  const box = Math.round(size * 0.46);
  const c = document.createElement("canvas");
  c.width = box;
  c.height = box;
  const ctx = c.getContext("2d")!;
  ctx.drawImage(icon, 0, 0, box, box);
  ctx.globalCompositeOperation = "source-in";
  ctx.fillStyle = color;
  ctx.fillRect(0, 0, box, box);
  tintCache.set(key, c);
  return c;
}

/** Draws one full frame (background and icon) onto `ctx`. */
export function drawThemeFrame(ctx: CanvasRenderingContext2D, theme: ThemeId, t: number, size: number, seed: number, icon: CanvasImageSource | null) {
  drawThemeBackground(ctx, theme, t, size, seed);
  drawIcon(ctx, theme, t, size, icon);
}

/** Turns an SVG string into an image the canvas can draw. */
export function svgToImage(svg: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("Could not draw the icon"));
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  });
}

/** All frames of one key, as base64 PNG without the data URL prefix. */
export function renderThemeFrames(theme: ThemeId, size: number, seed: number, icon: CanvasImageSource | null): string[] {
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Could not draw the theme");
  const frames: string[] = [];
  for (let i = 0; i < THEME_FRAMES; i++) {
    ctx.clearRect(0, 0, size, size);
    drawThemeFrame(ctx, theme, i / THEME_FRAMES, size, seed, icon);
    frames.push(canvas.toDataURL("image/png").split(",")[1] ?? "");
  }
  return frames;
}
