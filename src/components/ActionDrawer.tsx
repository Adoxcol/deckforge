import React from "react";
import { ChevronDown, ChevronUp, Plus, Search, Star, X } from "lucide-react";
import { actionNeedsParam } from "./ActionBinding";
import { DeckIcon } from "./DeckIcon";
import type { ActionTemplate } from "../data/actionCatalog";
import type { EncoderConfig, KeyConfig, TouchZoneConfig } from "../types/deckforge";

const CATEGORIES = ["All", "Custom", "Shortcuts", "Macro", "OBS", "Media", "System", "Navigation"] as const;

type CatalogApi = { open: boolean; show: () => void; hide: () => void };
const CatalogContext = React.createContext<CatalogApi | null>(null);

/** Shares whether the action grid under the deck is open. */
export function CatalogProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = React.useState(false);
  React.useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open]);
  const api = React.useMemo<CatalogApi>(
    () => ({ open, show: () => setOpen(true), hide: () => setOpen(false) }),
    [open],
  );
  return <CatalogContext.Provider value={api}>{children}</CatalogContext.Provider>;
}

export function useCatalog() {
  return React.useContext(CatalogContext);
}

interface ActionDrawerProps {
  actionCategoryFilter: string;
  activeEncoder: EncoderConfig | null;
  activeKey: KeyConfig;
  activeZone: TouchZoneConfig | null;
  actions: ActionTemplate[];
  setActionCategoryFilter: React.Dispatch<React.SetStateAction<string>>;
  surface: "key" | "zone" | "strip" | "encoder";
  triggerToast: (text: string, kind?: "success" | "info" | "warning") => void;
  updateSelectedEncoder: (index: number, patch: Partial<EncoderConfig>) => void;
  updateSelectedKey: (patch: Partial<KeyConfig>) => void;
  updateSelectedZone: (index: number, patch: Partial<TouchZoneConfig>) => void;
}

/** Drag payload type for an action card dropped on a key. */
export const ACTION_DRAG_TYPE = "application/x-deckforge-action";

/** The key fields an action fills in, shared by click-to-assign and drag-to-assign. */
export const keyPatchForAction = (act: ActionTemplate): Partial<KeyConfig> => ({
  title: act.title,
  subtitle: act.subtitle,
  iconName: act.iconName,
  actionId: act.id,
  actionLabel: act.actionLabel,
  actionCategory: act.category,
  actionParam: act.defaultParam,
  accentColor: act.accentColor,
  bgColor: act.bgColor,
});

/** What a touch zone takes from an action, for click or drop. */
export const zonePatchForAction = (act: ActionTemplate): Partial<TouchZoneConfig> => ({
  label: act.title,
  valueText: act.subtitle,
  tapActionId: act.id,
  tapActionParam: actionNeedsParam(act.id) ? act.defaultParam : "",
  accentColor: act.accentColor,
});

export type KnobPart = "ccw" | "push" | "cw";

/** What a knob takes from an action: its press (which also names the knob) or one turn direction. */
export const encoderPatchForAction = (act: ActionTemplate, part: KnobPart = "push"): Partial<EncoderConfig> => {
  const param = actionNeedsParam(act.id) ? act.defaultParam : "";
  if (part === "cw") return { cwAction: act.id, cwParam: param };
  if (part === "ccw") return { ccwAction: act.id, ccwParam: param };
  return { label: act.title, pushAction: act.id, pushParam: param };
};

/** Actions in a category whose name, label or description contains the search. */
function filterActions(actions: ActionTemplate[], category: string, search: string): ActionTemplate[] {
  const q = search.trim().toLowerCase();
  return actions.filter(
    (a) =>
      (category === "All" || a.category.toLowerCase() === category.toLowerCase()) &&
      (!q || a.title.toLowerCase().includes(q) || a.description.toLowerCase().includes(q) || a.actionLabel.toLowerCase().includes(q))
  );
}

/**
 * The action list under the deck. Memoized, with its own search state, so typing in the search box
 * and the deck's live updates don't redraw the whole editor (or this list) on every change.
 * App must pass stable callbacks.
 */
export const ActionDrawer = React.memo(function ActionDrawer({
  actionCategoryFilter,
  activeEncoder,
  activeKey,
  activeZone,
  actions,
  setActionCategoryFilter,
  surface,
  triggerToast,
  updateSelectedEncoder,
  updateSelectedKey,
  updateSelectedZone,
}: ActionDrawerProps) {
  const catalog = useCatalog();
  const open = catalog?.open ?? false;
  const show = catalog?.show ?? (() => {});
  const hide = catalog?.hide ?? (() => {});
  const [actionSearch, setActionSearch] = React.useState("");
  const deferredSearch = React.useDeferredValue(actionSearch);
  const filteredActions = React.useMemo(
    () => filterActions(actions, actionCategoryFilter, deferredSearch),
    [actions, actionCategoryFilter, deferredSearch]
  );
  const assignAction = (act: ActionTemplate) => {
    if (surface === "zone" && activeZone) {
      updateSelectedZone(activeZone.index, zonePatchForAction(act));
      triggerToast(`Assigned "${act.title}" to zone #${activeZone.index + 1}`, "info");
      return;
    }
    if (surface === "encoder" && activeEncoder) {
      updateSelectedEncoder(activeEncoder.index, encoderPatchForAction(act));
      triggerToast(`Assigned "${act.title}" to the knob press`, "info");
      return;
    }
    updateSelectedKey(keyPatchForAction(act));
    triggerToast(
      `Assigned "${act.actionLabel}" to Key #${activeKey.index + 1}`,
      "info"
    );
  };
  const assignTargetLabel =
    surface === "zone" && activeZone
      ? `Zone #${activeZone.index + 1}`
      : surface === "encoder" && activeEncoder
      ? `Knob #${activeEncoder.index + 1}`
      : `Key #${activeKey.index + 1}`;
  return (
    <div className={`bg-[var(--bg-sidebar)] border-t border-[var(--border-subtle)] shrink-0 flex flex-col ${open ? "h-[232px] shadow-[0_-8px_24px_rgba(0,0,0,0.28)]" : ""}`}>
      <div className="flex items-center gap-2 px-3 py-2 min-w-0">
        <div className="relative w-44 shrink-0">
          <Search
            size={14}
            className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none"
          />
          <input
            aria-label="Search actions"
            aria-expanded={open}
            value={actionSearch}
            onFocus={show}
            onClick={show}
            onChange={(e) => {
              setActionSearch(e.target.value);
              show();
            }}
            placeholder="Search actions..."
            className="w-full h-8 pl-8 pr-7 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)] transition-colors"
          />
          {actionSearch && (
            <button
              type="button"
              aria-label="Clear search"
              onClick={() => setActionSearch("")}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
            >
              <X size={12} />
            </button>
          )}
        </div>
        <div className="flex items-center gap-1 overflow-x-auto min-w-0" role="toolbar" aria-label="Action categories">
          {CATEGORIES.map((cat) => {
            const isAct = actionCategoryFilter === cat;
            return (
              <button
                key={cat}
                type="button"
                aria-pressed={isAct}
                onClick={() => {
                  setActionCategoryFilter(cat);
                  show();
                }}
                className={`shrink-0 h-7 px-2.5 rounded-md text-[11.5px] cursor-pointer transition-all inline-flex items-center gap-1 border ${
                  isAct
                    ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/40 text-[var(--text-primary)] font-semibold shadow-xs"
                    : "border-transparent text-[var(--text-secondary)] hover:bg-[var(--bg-panel)] hover:text-[var(--text-primary)]"
                }`}
              >
                {cat === "Custom" && <Star size={11} className="shrink-0 text-[var(--accent-text)]" />}
                {cat === "All" ? "All" : cat === "Custom" ? "Custom" : cat}
              </button>
            );
          })}
        </div>
        <div className="ml-auto flex items-center gap-1.5 shrink-0">
          <span className="hidden lg:inline-flex items-center px-2 h-6 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[10.5px] font-mono text-[var(--text-muted)]">
            Target: <span className="ml-1 text-[var(--text-secondary)]">{assignTargetLabel}</span>
          </span>
          <button
            type="button"
            onClick={() => (open ? hide() : show())}
            aria-expanded={open}
            aria-label={open ? "Collapse actions drawer" : "Browse actions"}
            title={open ? "Collapse actions (Esc)" : "Browse all actions"}
            className="h-7 px-2 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)]/70 hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] text-[11px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] inline-flex items-center gap-1 cursor-pointer transition-colors"
          >
            <span className="tabular-nums font-mono text-[10.5px] text-[var(--text-muted)]">{filteredActions.length}</span>
            {open ? <ChevronDown size={13} /> : <ChevronUp size={13} />}
          </button>
        </div>
      </div>

      {open && (
      <div className="min-h-0 flex-1 px-3 pb-3 overflow-hidden animate-fade-in">
        <div className="h-full grid grid-cols-[repeat(auto-fill,minmax(195px,1fr))] gap-2 overflow-y-auto pr-1 content-start">
          {filteredActions.length === 0 ? (
            <div className="col-span-full py-6 text-center text-xs text-[var(--text-muted)]">
              No actions match “{actionSearch.trim()}”{actionCategoryFilter !== "All" ? ` in ${actionCategoryFilter}` : ""}.
            </div>
          ) : (
            filteredActions.map((act) => (
              <div
                key={act.id}
                role="button"
                tabIndex={0}
                draggable
                title="Click to assign, or drag onto a key, touch zone or knob"
                onDragStart={(event) => {
                  event.dataTransfer.setData(ACTION_DRAG_TYPE, act.id);
                  event.dataTransfer.effectAllowed = "copy";
                }}
                onClick={() => assignAction(act)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    assignAction(act);
                  }
                }}
                className="group p-2 rounded-[var(--radius-md)] bg-[var(--bg-panel)]/75 border border-[var(--border-subtle)]/80 hover:border-[var(--accent-primary)]/50 hover:bg-[var(--bg-elevated)] focus-visible:border-[var(--accent-primary)] focus-visible:bg-[var(--bg-elevated)] hover:-translate-y-[1px] hover:shadow-md flex items-center justify-between gap-2 cursor-grab active:cursor-grabbing transition-all duration-150"
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <div
                    className="w-8 h-8 rounded-[var(--radius-sm)] flex items-center justify-center shrink-0 transition-transform duration-150 group-hover:scale-105"
                    style={{
                      backgroundColor: act.accentColor ? `${act.accentColor}1C` : "var(--bg-elevated)",
                      border: `1px solid ${act.accentColor ? `${act.accentColor}44` : "var(--border-subtle)"}`,
                    }}
                  >
                    <DeckIcon name={act.iconName} size={16} color={act.accentColor || "var(--text-primary)"} />
                  </div>
                  <div className="min-w-0">
                    <div className="text-[12.5px] font-medium text-[var(--text-primary)] truncate">
                      {act.title}
                    </div>
                    <div className="text-[11px] text-[var(--text-muted)] truncate">
                      {act.description}
                    </div>
                  </div>
                </div>
                <Plus size={13} className="shrink-0 text-[var(--text-muted)] opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity" aria-hidden />
              </div>
            ))
          )}
        </div>
      </div>
      )}
    </div>
  );
});
