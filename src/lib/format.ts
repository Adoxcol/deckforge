/** Small helpers shared by the editor screens. */

export function errorText(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return "Could not read that image";
}

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

export function buildPulseFrames(): string[] {
  const frames: string[] = [];
  for (let i = 0; i < 8; i += 1) {
    const canvas = document.createElement("canvas");
    canvas.width = 112;
    canvas.height = 112;
    const ctx = canvas.getContext("2d");
    if (!ctx) continue;
    ctx.fillStyle = "#1A1028";
    ctx.fillRect(0, 0, 112, 112);
    const radius = 22 + Math.sin((i / 8) * Math.PI * 2) * 10;
    ctx.fillStyle = `hsl(${262 + i * 8} 85% 64%)`;
    ctx.beginPath();
    ctx.arc(56, 46, Math.max(8, radius), 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#F4F5FA";
    ctx.font = "bold 14px sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("PULSE", 56, 96);
    frames.push(canvas.toDataURL("image/jpeg", 0.86));
  }
  return frames;
}
