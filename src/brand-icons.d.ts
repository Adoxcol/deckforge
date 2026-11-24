declare module "virtual:brand-icons" {
  const icons: Pick<import("simple-icons").SimpleIcon, "title" | "slug" | "hex" | "path">[];
  export default icons;
}

declare module "virtual:app-brand-icons" {
  const icons: Record<string, Pick<import("simple-icons").SimpleIcon, "hex" | "path">>;
  export default icons;
}
