import React from "react";
import { Search, Star, Upload, X } from "lucide-react";
import type { MediaPreview } from "../types/deckforge";
import { deckforgeApi } from "../bridge/tauriBridge";
import { bytesToBase64, errorText } from "../lib/format";
import { LIBRARY_PREFS_KEY, libraryItems, parseLibraryPrefs, type LibraryPrefs } from "../lib/mediaLibrary";
import { starterMedia } from "../lib/starterMedia";
import { ScreenFace } from "./ScreenFace";

interface Props {
  clips: Record<string, MediaPreview>;
  onStored: (clip: MediaPreview) => void;
  onUse: (clip: MediaPreview) => void;
  onClose: () => void;
}
const button = "rounded-md border border-[var(--border-subtle)] bg-[var(--bg-input)] px-3 py-2 text-xs disabled:opacity-50 hover:bg-[var(--bg-elevated)]";

export function MediaLibrary({ clips, onStored, onUse, onClose }: Props) {
  const dialog = React.useRef<HTMLDialogElement>(null);
  const upload = React.useRef<HTMLInputElement>(null);
  const busyRef = React.useRef(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState("");
  const [query, setQuery] = React.useState("");
  const [tab, setTab] = React.useState("Starter pack");
  const [url, setUrl] = React.useState("");
  const [limit, setLimit] = React.useState(24);
  const [hovered, setHovered] = React.useState<string | null>(null);
  const [selected, setSelected] = React.useState<MediaPreview | null>(null);
  const [starters, setStarters] = React.useState<MediaPreview[]>([]);
  const [prefs, setPrefs] = React.useState<LibraryPrefs>(() => {
    try { return parseLibraryPrefs(localStorage.getItem(LIBRARY_PREFS_KEY)); }
    catch { return { favorites: [], recent: [] }; }
  });
  React.useEffect(() => {
    dialog.current?.showModal();
    try { setStarters(starterMedia()); } catch (err) { setError(errorText(err)); }
    return () => dialog.current?.close();
  }, []);
  React.useEffect(() => { setLimit(24); }, [tab, query]);
  const all = React.useMemo(() => [...starters, ...Object.values(clips).filter(clip => !starters.some(s => s.id === clip.id))], [clips, starters]);
  const shown = libraryItems(tab === "Starter pack" ? starters : tab === "My media" ? Object.values(clips) : all, query, tab, prefs);
  const savePrefs = (next: LibraryPrefs) => {
    setPrefs(next);
    try { localStorage.setItem(LIBRARY_PREFS_KEY, JSON.stringify(next)); }
    catch { setError("Favorites and recent items could not be saved on this computer."); }
  };
  const run = async (work: () => Promise<void>) => {
    if (busyRef.current) return;
    busyRef.current = true; setBusy(true); setError("");
    try { await work(); } catch (err) { setError(errorText(err)); }
    finally { busyRef.current = false; setBusy(false); }
  };
  const imported = (clip: MediaPreview) => {
    onStored(clip); setSelected(clip); setTab("My media"); setQuery("");
  };
  const useSelected = () => run(async () => {
    if (!selected) return;
    let clip = clips[selected.id] ?? selected;
    if (selected.id.startsWith("starter-v1-") && !clips[selected.id]) {
      clip = await deckforgeApi.stageFrameSequence(selected.id, selected.fileName, selected.frames.map(frame => frame.split(",")[1]), 100);
      onStored(clip);
    }
    savePrefs({ ...prefs, recent: [clip.id, ...prefs.recent.filter(id => id !== clip.id)].slice(0, 50) });
    onUse(clip); onClose();
  });
  return <dialog ref={dialog} aria-labelledby="media-library-title" onCancel={event => { event.preventDefault(); if (!busyRef.current) onClose(); }}
    className="m-auto w-[760px] max-w-[95vw] max-h-[90vh] rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-panel)] text-[var(--text-primary)] p-0 shadow-2xl backdrop:bg-black/60">
    <div className="p-5 flex flex-col gap-4">
      <div className="flex items-center justify-between"><div><h2 id="media-library-title" className="text-lg font-bold">Media library</h2><p className="text-xs text-[var(--text-secondary)]">Animated icons and images, ready for your deck.</p></div><button className={button} disabled={busy} onClick={onClose} aria-label="Close media library"><X size={16}/></button></div>
      <div className="flex gap-2 flex-wrap">{["Starter pack", "My media", "Favorites", "Recent"].map(name => <button key={name} className={button} aria-pressed={tab === name} style={tab === name ? { borderColor: "var(--accent-primary)", color: "var(--accent-text)" } : undefined} onClick={() => setTab(name)}>{name}</button>)}</div>
      <div className="flex items-center gap-2"><Search size={16}/><input autoFocus aria-label="Search media library" placeholder="Search your library..." value={query} onChange={e => setQuery(e.target.value)} className="min-w-0 flex-1 rounded-md bg-[var(--bg-input)] px-3 py-2 text-sm"/><button className={button} disabled={busy} onClick={() => upload.current?.click()}><Upload size={14} className="inline mr-1"/>Upload</button></div>
      <input ref={upload} type="file" hidden accept="image/png,image/jpeg,image/gif,image/webp,image/bmp" onChange={event => {
        const file = event.target.files?.[0]; event.target.value = "";
        if (file) void run(async () => {
          if (file.size > 8 * 1024 * 1024) throw new Error("Image must be 8 MB or smaller.");
          imported(await deckforgeApi.stageScreenMedia(`media-${crypto.randomUUID()}`, file.name, bytesToBase64(new Uint8Array(await file.arrayBuffer()))));
        });
      }}/>
      <form className="flex gap-2" onSubmit={event => { event.preventDefault(); void run(async () => { imported(await deckforgeApi.importMediaUrl(url.trim())); setUrl(""); }); }}>
        <input aria-label="Direct image or GIF link" type="url" required placeholder="Paste a direct HTTPS image or GIF link" value={url} onChange={e => setUrl(e.target.value)} className="min-w-0 flex-1 rounded-md bg-[var(--bg-input)] px-3 py-2 text-xs"/>
        <button className={button} disabled={busy || !url.trim()}>Import link</button>
      </form>
      <p className="text-[11px] text-[var(--text-muted)]">Import files you own or have permission to use. Up to 8 MB. Webpage links are not image files.</p>
      {error && <p role="alert" className="text-xs text-[var(--status-warning)]">{error}</p>}
      {busy && <p role="status" className="text-xs">Preparing media...</p>}
      <div className="grid grid-cols-4 sm:grid-cols-6 gap-2 max-h-[35vh] overflow-y-auto p-1">
        {shown.slice(0, limit).map(clip => <div key={clip.id} className="relative">
          <button disabled={busy} aria-pressed={selected?.id === clip.id} className="w-full rounded-lg border p-2 text-left disabled:opacity-50" style={{ borderColor: selected?.id === clip.id ? "var(--accent-primary)" : "var(--border-subtle)" }} onClick={() => setSelected(clip)} onMouseEnter={() => setHovered(clip.id)} onMouseLeave={() => setHovered(null)} onFocus={() => setHovered(clip.id)} onBlur={() => setHovered(null)}>
            <div className="aspect-square overflow-hidden rounded bg-black/30">{hovered === clip.id ? <ScreenFace clip={clip} iconName="Image" accent="#A78BFA"/> : <img src={clip.frames[0]} alt="" loading="lazy" className="w-full h-full object-cover"/>}</div>
            <span className="block truncate text-[11px] mt-2" title={clip.fileName}>{clip.fileName}</span><span className="text-[10px] text-[var(--text-muted)]">{clip.frameCount > 1 ? "Animated" : "Image"}</span>
          </button>
          <button aria-label={`${prefs.favorites.includes(clip.id) ? "Unfavorite" : "Favorite"} ${clip.fileName}`} aria-pressed={prefs.favorites.includes(clip.id)} onClick={() => savePrefs({ ...prefs, favorites: prefs.favorites.includes(clip.id) ? prefs.favorites.filter(id => id !== clip.id) : [...prefs.favorites, clip.id] })} className="absolute top-2 right-2 rounded bg-black/70 p-1"><Star size={13} fill={prefs.favorites.includes(clip.id) ? "#FBBF24" : "none"}/></button>
        </div>)}
        {!shown.length && <p className="col-span-full py-8 text-center text-sm text-[var(--text-secondary)]">{query ? "No matching media." : "Nothing here yet. Choose a starter animation or import an image or GIF."}</p>}
      </div>
      {shown.length > limit && <button className={button} onClick={() => setLimit(n => n + 24)}>Show more ({shown.length - limit} remaining)</button>}
      <div className="flex gap-3 items-center border-t border-[var(--border-subtle)] pt-3">
        <div className="h-16 w-16 shrink-0 rounded bg-black/30 overflow-hidden">{selected && <ScreenFace clip={selected} iconName="Image" accent="#A78BFA"/>}</div>
        <div className="flex-1 min-w-0"><p className="text-sm truncate">{selected?.fileName ?? "Choose an image or animation"}</p><p className="text-[11px] text-[var(--text-muted)]">Adjust fit and speed in the editor. Apply to Device when ready.</p></div>
        <button className={button + " text-[var(--accent-text)]"} disabled={!selected || busy} onClick={() => void useSelected()}>Use on screen</button>
      </div>
      <p className="text-[10px] text-[var(--text-muted)]">Starter pack: original DeckForge animations, CC0. Imported media stays on this computer for offline use.</p>
    </div>
  </dialog>;
}
