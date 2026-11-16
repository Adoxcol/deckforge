/** Keep the full searchable catalog, but ship only fields the picker consumes. */
export function brandIconsPlugin() {
  const id = "virtual:brand-icons";
  const resolved = "\0" + id;
  return {
    name: "deckforge-brand-icons",
    resolveId(source) {
      return source === id ? resolved : null;
    },
    async load(source) {
      if (source !== resolved) return null;
      const icons = await import("simple-icons");
      const catalog = Object.values(icons).map(({ title, slug, hex, path }) => ({ title, slug, hex, path }));
      return `export default ${JSON.stringify(catalog)};`;
    },
  };
}
