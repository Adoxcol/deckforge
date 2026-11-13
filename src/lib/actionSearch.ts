export interface SearchableAction {
  id: string;
  actionLabel: string;
  title?: string;
  category?: string;
  description?: string;
}

/** The group an action is listed under: its category, else the text before the colon. */
export function groupOf(choice: SearchableAction): string {
  if (choice.category) return choice.category;
  const colon = choice.actionLabel.indexOf(":");
  return colon > 0 ? choice.actionLabel.slice(0, colon) : "Other";
}

/** The label without its "Group:" prefix. */
export function nameOf(choice: SearchableAction): string {
  const colon = choice.actionLabel.indexOf(":");
  return colon > 0 ? choice.actionLabel.slice(colon + 1).trim() : choice.actionLabel;
}

/** Every word typed must appear somewhere in the action's name, group, description or id. */
export function matchesQuery(choice: SearchableAction, query: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return true;
  const hay = `${choice.actionLabel} ${choice.title ?? ""} ${groupOf(choice)} ${choice.description ?? ""} ${choice.id}`.toLowerCase();
  return words.every((word) => hay.includes(word));
}
