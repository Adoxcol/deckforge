import React from "react";
import { Search, X } from "lucide-react";
import { actionNeedsParam } from "./ActionBinding";
import { DeckIcon } from "./DeckIcon";
import type { ActionTemplate } from "../data/actionCatalog";
import type { EncoderConfig, KeyConfig, TouchZoneConfig } from "../types/deckforge";

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
  return (
    <div className="h-[214px] bg-[var(--bg-sidebar)] border-t border-[var(--border-subtle)] grid grid-cols-[172px_1fr] p-3 gap-4 shrink-0">
      {/* Left Category Filter Rail */}
      <div className="border-r border-[var(--border-subtle)] pr-3 flex flex-col gap-0.5 overflow-y-auto">
        <div className="text-[11px] font-semibold tracking-wide text-[var(--text-muted)] px-2 mb-1">Actions</div>
        {["All", "Custom", "Shortcuts", "Macro", "OBS", "Media", "System", "Navigation"].map(
          (cat) => {
            const isAct = actionCategoryFilter === cat;
            const count = cat === "All"
              ? actions.length
              : actions.filter((a) => a.category.toLowerCase() === cat.toLowerCase()).length;
            return (
              <button
                key={cat}
                type="button"
                aria-pressed={isAct}
                onClick={() => setActionCategoryFilter(cat)}
                className={`flex items-center justify-between text-left px-2.5 py-1.5 rounded-md text-[11.5px] cursor-pointer transition-colors ${
                  isAct
                    ? "bg-[var(--accent-soft)] text-[var(--text-primary)] font-semibold"
                    : "text-[var(--text-secondary)] hover:bg-[var(--bg-panel)] hover:text-[var(--text-primary)]"
                }`}
              >
                <span className="truncate">{cat === "All" ? "All Actions" : cat === "Custom" ? "★ Custom Actions" : cat}</span>
                <span className={`text-[10px] font-mono tabular-nums ${isAct ? "text-[var(--accent-text)]" : "text-[var(--text-muted)]"}`}>{count}</span>
              </button>
            );
          }
        )}
      </div>

      {/* Right Search + Action Cards Grid */}
      <div className="flex flex-col gap-2.5 overflow-hidden">
        <div className="relative">
          <Search
            size={14}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none"
          />
          <input
            aria-label="Search actions"
            value={actionSearch}
            onChange={(e) => setActionSearch(e.target.value)}
            placeholder="Search actions..."
            className="w-full h-8 pl-8 pr-8 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none placeholder:text-[var(--text-muted)] transition-colors"
          />
          {actionSearch && (
            <button
              type="button"
              aria-label="Clear search"
              onClick={() => setActionSearch("")}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
            >
              <X size={12} />
            </button>
          )}
        </div>

        <div className="grid grid-cols-[repeat(auto-fill,minmax(195px,1fr))] gap-1.5 overflow-y-auto pr-1 pt-0.5 content-start">
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
                className="group p-2 rounded-[var(--radius-md)] bg-[var(--bg-panel)]/75 border border-[var(--border-subtle)]/80 hover:border-[var(--accent-primary)]/50 hover:bg-[var(--bg-elevated)] hover:-translate-y-[1px] hover:shadow-md flex items-center justify-between gap-2.5 cursor-grab active:cursor-grabbing transition-all duration-150"
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
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
});
