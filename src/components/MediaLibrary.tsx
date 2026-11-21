import React from "react";
import { RefreshCw, Search, Star, Upload, X } from "lucide-react";
import type { CommonsGif, MediaPreview } from "../types/deckforge";
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
const button = "rounded-md border border-[var(--border-subtle)] bg-[var(--bg-input)] px-3 py-2 text-xs font-medium disabled:opacity-50 disabled:cursor-default hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] cursor-pointer transition-colors";

export function MediaLibrary({ clips, onStored, onUse, onClose }: Props) {
  const dialog = React.useRef<HTMLDialogElement>(null);
  const upload = React.useRef<HTMLInputElement>(null);
  const busyRef = React.useRef(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState("");
  const [query, setQuery] = React.useState("");
  const [tab, setTab] = React.useState("Starter pack");
  const [url, setUrl] = React.useState("");
  const [gifQuery, setGifQuery] = React.useState("");
  const [gifSearched, setGifSearched] = React.useState("");
  const [gifResults, setGifResults] = React.useState<CommonsGif[]>([]);
  const [gifSelected, setGifSelected] = React.useState<CommonsGif | null>(null);
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
  const useGif = () => run(async () => {
    if (!gifSelected) return;
    const clip = await deckforgeApi.importMediaUrl(gifSelected.imageUrl);
    onStored(clip);
    savePrefs({ ...prefs, recent: [clip.id, ...prefs.recent.filter(id => id !== clip.id)].slice(0, 50) });
    onUse(clip); onClose();
  });
  return <dialog ref={dialog} aria-labelledby="media-library-title" onCancel={event => { event.preventDefault(); if (!busyRef.current) onClose(); }}
    className="m-auto w-[760px] max-w-[95vw] max-h-[90vh] flex flex-col overflow-hidden rounded-2xl border border-[var(--border-strong)] bg-[var(--bg-panel)] text-[var(--text-primary)] p-0 shadow-2xl backdrop:bg-black/65 backdrop:backdrop-blur-[2px] animate-pop-in">
    <div className="p-5 flex-1 min-h-0 overflow-y-auto flex flex-col gap-4">
      <div className="flex items-center justify-between"><div><h2 id="media-library-title" className="text-lg font-bold">Media library</h2><p className="text-xs text-[var(--text-secondary)]">Animated icons and images, ready for your deck.</p></div><button type="button" className={button} disabled={busy} onClick={onClose} aria-label="Close media library"><X size={16}/></button></div>
      <div role="tablist" aria-label="Media library tabs" className="flex gap-1 flex-wrap bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] p-1 rounded-lg w-fit">{["Starter pack", "My media", "Favorites", "Recent", "Find GIFs"].map(name => <button type="button" role="tab" key={name} className={`px-3 py-1.5 rounded-md text-xs font-medium cursor-pointer transition-all border ${tab === name ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-white font-semibold shadow-xs" : "border-transparent text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)]"}`} aria-selected={tab === name} onClick={() => setTab(name)}>{name}</button>)}</div>
      {tab === "Find GIFs" ? <form className="flex items-center gap-2" onSubmit={event => { event.preventDefault(); void run(async () => { const term = gifQuery.trim(); setGifResults([]); setGifSelected(null); setGifSearched(term); setGifResults(await deckforgeApi.searchCommonsGifs(term)); }); }}>
        <div className="min-w-0 flex-1 flex items-center gap-2 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus-within:border-[var(--accent-primary)] px-3 h-9 transition-colors">
          <Search size={15} className="text-[var(--text-muted)] shrink-0"/>
          <input aria-label="Search Wikimedia Commons GIFs" placeholder="Search GIFs on Wikimedia Commons..." value={gifQuery} onChange={e => setGifQuery(e.target.value)} className="min-w-0 flex-1 bg-transparent outline-none text-sm text-[var(--text-primary)] placeholder:text-[var(--text-muted)]"/>
          {gifQuery && <button type="button" aria-label="Clear GIF search" onClick={() => setGifQuery("")} className="p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"><X size={13}/></button>}
        </div>
        <button type="submit" className={button} disabled={busy || gifQuery.trim().length < 2}>Search</button>
      </form> : <div className="flex items-center gap-2"><div className="min-w-0 flex-1 flex items-center gap-2 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus-within:border-[var(--accent-primary)] px-3 h-9 transition-colors"><Search size={15} className="text-[var(--text-muted)] shrink-0"/><input autoFocus aria-label="Search media library" placeholder="Search your library..." value={query} onChange={e => setQuery(e.target.value)} className="min-w-0 flex-1 bg-transparent outline-none text-sm text-[var(--text-primary)] placeholder:text-[var(--text-muted)]"/>{query && <button type="button" aria-label="Clear media search" onClick={() => setQuery("")} className="p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"><X size={13}/></button>}</div><button type="button" className={button} disabled={busy} onClick={() => upload.current?.click()}><Upload size={14} className="inline mr-1"/>Upload</button></div>}
      <input ref={upload} type="file" hidden accept="image/png,image/jpeg,image/gif,image/webp,image/bmp" onChange={event => {
        const file = event.target.files?.[0]; event.target.value = "";
        if (file) void run(async () => {
          if (file.size > 8 * 1024 * 1024) throw new Error("Image must be 8 MB or smaller.");
          imported(await deckforgeApi.stageScreenMedia(`media-${crypto.randomUUID()}`, file.name, bytesToBase64(new Uint8Array(await file.arrayBuffer()))));
        });
      }}/>
      <form className="flex gap-2" onSubmit={event => { event.preventDefault(); void run(async () => { imported(await deckforgeApi.importMediaUrl(url.trim())); setUrl(""); }); }}>
        <input aria-label="Direct image or GIF link" type="url" required placeholder="Paste a direct HTTPS image or GIF link" value={url} onChange={e => setUrl(e.target.value)} className="min-w-0 flex-1 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] outline-none px-3 py-2 text-xs text-[var(--text-primary)] placeholder:text-[var(--text-muted)]"/>
        <button type="submit" className={button} disabled={busy || !url.trim()}>Import link</button>
      </form>
      <p className="text-[11px] text-[var(--text-muted)]">Import files you own or have permission to use. Up to 8 MB. Webpage links are not image files.</p>
      {error && <p role="alert" className="text-xs text-[var(--status-warning)]">{error}</p>}
      {busy && <p role="status" className="text-xs text-[var(--accent-text)] flex items-center gap-1.5"><RefreshCw size={12} className="animate-spin"/>Preparing media...</p>}
      {tab === "Find GIFs" ? <>
        <p className="text-[11px] text-[var(--text-muted)]">Search Wikimedia Commons. Each GIF has its own license; check its file page before sharing or publishing it.</p>
        <div className="grid grid-cols-4 sm:grid-cols-6 gap-2 max-h-[35vh] overflow-y-auto p-1">
          {gifResults.map(gif => <button type="button" key={gif.sourceUrl} disabled={busy} aria-pressed={gifSelected?.sourceUrl === gif.sourceUrl}
            className="rounded-lg border bg-[var(--bg-sidebar)] hover:bg-[var(--bg-elevated)] p-2 text-left disabled:opacity-50 cursor-pointer transition-colors"
            style={{ borderColor: gifSelected?.sourceUrl === gif.sourceUrl ? "var(--accent-primary)" : "var(--border-subtle)" }} onClick={() => setGifSelected(gif)}>
            <span className="block aspect-square overflow-hidden rounded bg-black/30"><img src={gif.thumbnailUrl} alt="" loading="lazy" className="w-full h-full object-cover"/></span>
            <span className="block truncate text-[11px] mt-2" title={gif.title}>{gif.title}</span>
            <span className="text-[10px] text-[var(--text-muted)]">{(gif.size / 1024 / 1024).toFixed(1)} MB</span>
          </button>)}
          {!gifResults.length && !error && <p className="col-span-full py-8 text-center text-sm text-[var(--text-secondary)]">{gifSearched ? `No usable GIFs found for “${gifSearched}”.` : "Search for an animated GIF to use on your deck."}</p>}
        </div>
        <div className="flex gap-3 items-center border-t border-[var(--border-subtle)] pt-3">
          <div className="h-16 w-16 shrink-0 rounded-lg bg-black/30 border border-[var(--border-subtle)] overflow-hidden">{gifSelected && <img src={gifSelected.imageUrl} alt="Selected GIF preview" className="w-full h-full object-cover"/>}</div>
          <div className="flex-1 min-w-0"><p className="text-sm truncate">{gifSelected?.title ?? "Choose a GIF"}</p>{gifSelected && <button type="button" className="text-[11px] text-[var(--accent-text)] hover:underline cursor-pointer" onClick={() => void deckforgeApi.openCommonsFilePage(gifSelected.sourceUrl).catch(err => setError(errorText(err)))}>Open file page and license</button>}</div>
          <button type="button" className="h-9 px-4 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer disabled:opacity-40 disabled:cursor-default transition-colors" disabled={!gifSelected || busy} onClick={() => void useGif()}>Use on screen</button>
        </div>
      </> : <>
      <div className="grid grid-cols-4 sm:grid-cols-6 gap-2 max-h-[35vh] overflow-y-auto p-1">
        {shown.slice(0, limit).map(clip => <div key={clip.id} className="relative group">
          <button type="button" disabled={busy} aria-pressed={selected?.id === clip.id} className="w-full rounded-lg border bg-[var(--bg-sidebar)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] p-2 text-left disabled:opacity-50 cursor-pointer transition-all" style={{ borderColor: selected?.id === clip.id ? "var(--accent-primary)" : "var(--border-subtle)" }} onClick={() => setSelected(clip)} onMouseEnter={() => setHovered(clip.id)} onMouseLeave={() => setHovered(null)} onFocus={() => setHovered(clip.id)} onBlur={() => setHovered(null)}>
            <div className="aspect-square overflow-hidden rounded bg-black/30">{hovered === clip.id ? <ScreenFace clip={clip} iconName="Image" accent="#A78BFA"/> : <img src={clip.frames[0]} alt="" loading="lazy" className="w-full h-full object-cover"/>}</div>
            <span className="block truncate text-[11px] mt-2" title={clip.fileName}>{clip.fileName}</span><span className="text-[10px] text-[var(--text-muted)]">{clip.frameCount > 1 ? "Animated" : "Image"}</span>
          </button>
          <button type="button" aria-label={`${prefs.favorites.includes(clip.id) ? "Unfavorite" : "Favorite"} ${clip.fileName}`} aria-pressed={prefs.favorites.includes(clip.id)} onClick={() => savePrefs({ ...prefs, favorites: prefs.favorites.includes(clip.id) ? prefs.favorites.filter(id => id !== clip.id) : [...prefs.favorites, clip.id] })} className={`absolute top-2 right-2 rounded bg-black/70 hover:bg-black/90 text-white p-1 cursor-pointer transition-all hover:scale-110 ${prefs.favorites.includes(clip.id) ? "opacity-100" : "opacity-60 group-hover:opacity-100 focus-visible:opacity-100"}`}><Star size={13} fill={prefs.favorites.includes(clip.id) ? "#FBBF24" : "none"}/></button>
        </div>)}
        {!shown.length && <p className="col-span-full py-8 text-center text-sm text-[var(--text-secondary)]">{query ? "No matching media." : "Nothing here yet. Choose a starter animation or import an image or GIF."}</p>}
      </div>
      {shown.length > limit && <button type="button" className={button} onClick={() => setLimit(n => n + 24)}>Show more ({shown.length - limit} remaining)</button>}
      <div className="flex gap-3 items-center border-t border-[var(--border-subtle)] pt-3">
        <div className="h-16 w-16 shrink-0 rounded bg-black/30 overflow-hidden">{selected && <ScreenFace clip={selected} iconName="Image" accent="#A78BFA"/>}</div>
        <div className="flex-1 min-w-0"><p className="text-sm truncate">{selected?.fileName ?? "Choose an image or animation"}</p><p className="text-[11px] text-[var(--text-muted)]">Adjust fit and speed in the editor. Apply to Device when ready.</p></div>
        <button type="button" className="h-9 px-4 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-xs font-semibold text-white cursor-pointer disabled:opacity-40 disabled:cursor-default" disabled={!selected || busy} onClick={() => void useSelected()}>Use on screen</button>
      </div>
      </>}
      <p className="text-[10px] text-[var(--text-muted)]">Starter pack: original DeckForge animations, CC0. Imported media stays on this computer for offline use.</p>
    </div>
  </dialog>;
}
