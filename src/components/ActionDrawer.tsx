import React from "react";
import { ChevronRight, Search } from "lucide-react";
import { actionNeedsParam } from "./ActionBinding";
import { DeckIcon } from "./DeckIcon";
import type { ActionTemplate } from "../data/actionCatalog";
import type { EncoderConfig, KeyConfig, TouchZoneConfig } from "../types/deckforge";

interface ActionDrawerProps {
  actionCategoryFilter: string;
  actionSearch: string;
  activeEncoder: EncoderConfig | null;
  activeKey: KeyConfig;
  activeZone: TouchZoneConfig | null;
  filteredActions: ActionTemplate[];
  setActionCategoryFilter: React.Dispatch<React.SetStateAction<string>>;
  setActionSearch: React.Dispatch<React.SetStateAction<string>>;
  surface: "key" | "zone" | "strip" | "encoder";
  triggerToast: (text: string, kind?: "success" | "info" | "warning") => void;
  updateSelectedEncoder: (index: number, patch: Partial<EncoderConfig>) => void;
  updateSelectedKey: (patch: Partial<KeyConfig>) => void;
  updateSelectedZone: (index: number, patch: Partial<TouchZoneConfig>) => void;
}

export const ActionDrawer: React.FC<ActionDrawerProps> = ({
  actionCategoryFilter,
  actionSearch,
  activeEncoder,
  activeKey,
  activeZone,
  filteredActions,
  setActionCategoryFilter,
  setActionSearch,
  surface,
  triggerToast,
  updateSelectedEncoder,
  updateSelectedKey,
  updateSelectedZone,
}) => {
  return (
    <div className="h-[205px] bg-[#10131D] border-t border-[#272D3D] grid grid-cols-[165px_1fr] p-3 gap-4 shrink-0">
      {/* Left Category Filter Rail */}
      <div className="border-r border-[#272D3D] pr-3 flex flex-col gap-0.5 overflow-y-auto">
        <div className="text-[11px] font-semibold text-[#A6AFC4] px-2 mb-1">Actions</div>
        {["All", "Custom", "Shortcuts", "Macro", "OBS", "Media", "System", "Navigation"].map(
          (cat) => {
            const isAct = actionCategoryFilter === cat;
            return (
              <button
                key={cat}
                onClick={() => setActionCategoryFilter(cat)}
                className={`text-left px-2.5 py-1.5 rounded-md text-[11.5px] cursor-pointer transition-colors ${
                  isAct
                    ? "bg-[#30244F] text-[#F4F5FA] font-semibold"
                    : "text-[#A6AFC4] hover:bg-[#151925] hover:text-white"
                }`}
              >
                {cat === "All" ? "All Actions" : cat === "Custom" ? "★ Custom Actions" : cat}
              </button>
            );
          }
        )}
      </div>

      {/* Right Search + 3x2 Action Cards Grid */}
      <div className="flex flex-col gap-2.5 overflow-hidden">
        <div className="relative">
          <Search
            size={14}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-[#707B94]"
          />
          <input
            value={actionSearch}
            onChange={(e) => setActionSearch(e.target.value)}
            placeholder="Search actions..."
            className="w-full h-8 pl-8 pr-3 rounded-md bg-[#111522] border border-[#272D3D] focus:border-[#8B5CF6] text-xs text-[#F4F5FA] outline-none"
          />
        </div>

        <div className="grid grid-cols-3 gap-2 overflow-y-auto pr-1">
          {filteredActions.map((act) => (
            <div
              key={act.id}
              onClick={() => {
                if (surface === "zone" && activeZone) {
                  updateSelectedZone(activeZone.index, {
                    label: act.title,
                    valueText: act.subtitle,
                    tapActionId: act.id,
                    tapActionParam: actionNeedsParam(act.id) ? act.defaultParam : "",
                    accentColor: act.accentColor,
                  });
                  triggerToast(`Assigned "${act.title}" to zone #${activeZone.index + 1}`, "info");
                  return;
                }
                if (surface === "encoder" && activeEncoder) {
                  updateSelectedEncoder(activeEncoder.index, {
                    label: act.title,
                    pushAction: act.id,
                    pushParam: actionNeedsParam(act.id) ? act.defaultParam : "",
                  });
                  triggerToast(`Assigned "${act.title}" to the knob press`, "info");
                  return;
                }
                updateSelectedKey({
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
                triggerToast(
                  `Assigned "${act.actionLabel}" to Key #${activeKey.index + 1}`,
                  "info"
                );
              }}
              className="p-2.5 rounded-lg bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6]/70 hover:bg-[#1C2130] flex items-center justify-between gap-2.5 cursor-pointer transition-all"
            >
              <div className="flex items-center gap-2.5 min-w-0">
                <div
                  className="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
                  style={{
                    backgroundColor: `${act.accentColor}20`,
                    border: `1px solid ${act.accentColor}50`,
                  }}
                >
                  <DeckIcon name={act.iconName} size={16} color={act.accentColor} />
                </div>
                <div className="min-w-0">
                  <div className="text-[12px] font-semibold text-[#F4F5FA] truncate">
                    {act.title}
                  </div>
                  <div className="text-[10.5px] text-[#707B94] truncate">
                    {act.description}
                  </div>
                </div>
              </div>
              <ChevronRight size={14} className="text-[#707B94] shrink-0" />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
