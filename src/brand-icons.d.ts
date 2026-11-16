declare module "virtual:brand-icons" {
  const icons: Pick<import("simple-icons").SimpleIcon, "title" | "slug" | "hex" | "path">[];
  export default icons;
}
