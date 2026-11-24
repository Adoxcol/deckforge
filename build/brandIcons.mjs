/** Keep the full searchable catalog lazy; app tiles use a much smaller fallback set. */
const APP_BRAND_SLUGS = {
  chrome: "googlechrome", discord: "discord", spotify: "spotify", obsidian: "obsidian",
  figma: "figma", notion: "notion", steam: "steam", steamwebhelper: "steam",
  dropbox: "dropbox", epicgameslauncher: "epicgames", riotclientservices: "riotgames",
  docker: "docker", "docker desktop": "docker", qbittorrent: "qbittorrent",
  nvidia: "nvidia", "nvidia share": "nvidia",
};

export function brandIconsPlugin() {
  const full = "\0virtual:brand-icons";
  const app = "\0virtual:app-brand-icons";
  return {
    name: "deckforge-brand-icons",
    resolveId(source) {
      return source === "virtual:brand-icons" ? full : source === "virtual:app-brand-icons" ? app : null;
    },
    async load(source) {
      if (source !== full && source !== app) return null;
      const icons = await import("simple-icons");
      if (source === app) {
        const bySlug = new Map(Object.values(icons).map(({ slug, hex, path }) => [slug, { hex, path }]));
        const byProcess = Object.fromEntries(Object.entries(APP_BRAND_SLUGS).map(([process, slug]) => [process, bySlug.get(slug)]));
        return `export default ${JSON.stringify(byProcess)};`;
      }
      const catalog = Object.values(icons).map(({ title, slug, hex, path }) => ({ title, slug, hex, path }));
      return `export default ${JSON.stringify(catalog)};`;
    },
  };
}
