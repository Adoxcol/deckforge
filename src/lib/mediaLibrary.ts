import type { MediaPreview } from "../types/deckforge";
export interface LibraryPrefs { favorites: string[]; recent: string[] }
export const LIBRARY_PREFS_KEY = "deckforge-media-library-v1";
export function parseLibraryPrefs(raw: string | null): LibraryPrefs {
  try {
    const value = JSON.parse(raw || "{}");
    const ids = (items: unknown) => Array.isArray(items) ? [...new Set(items.filter((id): id is string => typeof id === "string"))].slice(0, 500) : [];
    return { favorites: ids(value.favorites), recent: ids(value.recent).slice(0, 50) };
  } catch { return { favorites: [], recent: [] }; }
}
export function libraryItems(clips: MediaPreview[], query: string, tab: string, prefs: LibraryPrefs): MediaPreview[] {
  const words = query.toLowerCase().trim().split(/\s+/).filter(Boolean);
  const matches = clips.filter(clip => words.every(word => clip.fileName.toLowerCase().includes(word)));
  if (tab === "Favorites") return matches.filter(clip => prefs.favorites.includes(clip.id));
  if (tab === "Recent") return prefs.recent.flatMap(id => matches.find(clip => clip.id === id) ?? []);
  return matches.sort((a, b) => a.fileName.localeCompare(b.fileName));
}
