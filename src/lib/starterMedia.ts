import type { MediaPreview } from "../types/deckforge";
// Original DeckForge artwork, dedicated to the public domain under CC0-1.0.
export const STARTER_NAMES = ["Neon pulse", "Orbit", "Audio bars", "Rainbow wave"];
export function starterMedia(): MediaPreview[] {
  return STARTER_NAMES.map((fileName, kind) => {
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 112;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Your graphics renderer could not create the starter animations.");
    const frames = Array.from({ length: 16 }, (_, frame) => {
      const phase = frame / 16 * Math.PI * 2;
      ctx.fillStyle = "#101322"; ctx.fillRect(0, 0, 112, 112);
      ctx.strokeStyle = "#A78BFA"; ctx.fillStyle = "#A78BFA"; ctx.lineWidth = 5;
      if (kind === 0) {
        ctx.beginPath(); ctx.arc(56, 56, 24 + 8 * Math.sin(phase), 0, Math.PI * 2); ctx.stroke();
        ctx.fillStyle = "#F472B6"; ctx.beginPath(); ctx.arc(56, 56, 8, 0, Math.PI * 2); ctx.fill();
      } else if (kind === 1) {
        ctx.strokeStyle = "#334155"; ctx.beginPath(); ctx.arc(56, 56, 32, 0, Math.PI * 2); ctx.stroke();
        ctx.fillStyle = "#22D3EE"; ctx.beginPath(); ctx.arc(56 + 32 * Math.cos(phase), 56 + 32 * Math.sin(phase), 9, 0, Math.PI * 2); ctx.fill();
      } else {
        for (let x = 0; x < 7; x++) {
          const height = 16 + (1 + Math.sin(phase + x * 0.7)) * 24;
          ctx.fillStyle = `hsl(${kind === 2 ? 155 + x * 8 : x * 45 + frame * 22.5} 85% 65%)`;
          ctx.fillRect(12 + x * 13, 56 - height / 2, 8, height);
        }
      }
      return canvas.toDataURL("image/png");
    });
    return { id: `starter-v1-${kind}`, fileName, kind: "gif", width: 112, height: 112, frameCount: 16, sourceFrameCount: 16, frameDelayMs: 100, delaysMs: Array(16).fill(100), frames };
  });
}
