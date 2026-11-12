import React from "react";
import {
  Activity,
  ChevronDown,
  ChevronUp,
  Clock,
  Eye,
  FolderPlus,
  Layers,
  Play,
  Plus,
  Terminal,
  Trash2,
  Zap,
} from "lucide-react";
import type { MacroRunReport, MacroSequence, MacroStep, Profile } from "../types/deckforge";

type StepType = MacroStep["stepType"];

const BLOCKS: {
  type: StepType;
  title: string;
  hint: string;
  parameter: string;
  color: string;
  Icon: React.ComponentType<{ size?: number; color?: string }>;
}[] = [
  { type: "Delay", title: "Wait", hint: "Hold, then continue", parameter: "500", color: "#56B9F2", Icon: Clock },
  { type: "Shortcut", title: "Shortcut", hint: "Send a key chord", parameter: "Ctrl+Shift+M", color: "#8B5CF6", Icon: Terminal },
  { type: "LaunchApp", title: "Open app", hint: "Start a program", parameter: "notepad.exe", color: "#35C98B", Icon: FolderPlus },
  { type: "IfWindowExists", title: "If running", hint: "Check that an app is open", parameter: "obs64.exe", color: "#F4B860", Icon: Eye },
  { type: "SwitchProfile", title: "Switch deck", hint: "Change the deck profile", parameter: "profile-streaming", color: "#F16B79", Icon: Layers },
  { type: "HttpRequest", title: "Webhook", hint: "Call a URL", parameter: "https://", color: "#C084FC", Icon: Activity },
];

function blockFor(type: string) {
  return BLOCKS.find((block) => block.type === type) ?? BLOCKS[0];
}

function plannedMs(step: MacroStep): number {
  if (step.stepType === "Delay") {
    const ms = Number(step.parameter);
    return Number.isFinite(ms) ? Math.max(40, Math.min(3000, ms)) : 100;
  }
  return 160;
}

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(ms % 1000 === 0 ? 0 : 1)}s`;
  return `${Math.round(ms)}ms`;
}

export const MacroStudio: React.FC<{
  macros: MacroSequence[];
  profiles: Profile[];
  selectedMacroId: string;
  selectedStepIdx: number;
  runReport: MacroRunReport | null;
  activeKeyLabel: string | null;
  onSelectMacro: (id: string) => void;
  onSelectStep: (index: number) => void;
  onChange: (macros: MacroSequence[]) => void;
  onRun: (macro: MacroSequence) => void;
  onStop: () => void;
  onAssign: (macro: MacroSequence) => void;
}> = ({
  macros,
  profiles,
  selectedMacroId,
  selectedStepIdx,
  runReport,
  activeKeyLabel,
  onSelectMacro,
  onSelectStep,
  onChange,
  onRun,
  onStop,
  onAssign,
}) => {
  const macro = macros.find((item) => item.id === selectedMacroId) ?? macros[0];
  const step = macro?.steps[selectedStepIdx];
  const weights = macro?.steps.map(plannedMs) ?? [];
  const total = weights.reduce((sum, value) => sum + value, 0);

  const updateMacro = (recipe: (current: MacroSequence) => MacroSequence) => {
    if (!macro) return;
    onChange(macros.map((item) => (item.id === macro.id ? recipe(item) : item)));
  };

  const addBlock = (type: StepType) => {
    const kind = blockFor(type);
    const next: MacroStep = {
      id: `step-${Date.now()}`,
      stepType: type,
      label: kind.title,
      parameter: type === "SwitchProfile" ? profiles[0]?.id ?? kind.parameter : kind.parameter,
      timeoutMs: type === "Delay" ? 1000 : 2000,
      onError: type === "IfWindowExists" ? "Continue" : "Stop",
    };
    updateMacro((current) => ({ ...current, steps: [...current.steps, next], lastEditedMs: Date.now() }));
    onSelectStep(macro ? macro.steps.length : 0);
  };

  const createMacro = () => {
    const created: MacroSequence = {
      id: `macro-${Date.now()}`,
      name: "New chain",
      description: "A sequence this key will run from top to bottom.",
      steps: [],
      lastEditedMs: Date.now(),
    };
    onChange([created, ...macros]);
    onSelectMacro(created.id);
    onSelectStep(0);
  };

  return (
    <div className="flex-1 grid grid-cols-[250px_1fr_300px] overflow-hidden">
      <aside className="bg-[#10131D] border-r border-[#272D3D] p-3 flex flex-col gap-2 overflow-y-auto">
        <button
          type="button"
          onClick={createMacro}
          className="h-9 rounded-lg bg-[#8B5CF6] hover:bg-[#9D75F8] text-white text-xs font-semibold flex items-center justify-center gap-1.5 cursor-pointer"
        >
          <Plus size={14} />
          New chain
        </button>
        {macros.map((item) => {
          const bars = item.steps.map(plannedMs);
          const sum = bars.reduce((acc, value) => acc + value, 0) || 1;
          const active = item.id === macro?.id;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => {
                onSelectMacro(item.id);
                onSelectStep(0);
              }}
              className={`text-left p-3 rounded-xl border cursor-pointer ${
                active ? "bg-[#221A36] border-[#8B5CF6]" : "bg-[#151925] border-[#272D3D] hover:border-[#394158]"
              }`}
            >
              <div className="text-xs font-bold text-[#F4F5FA] truncate">{item.name}</div>
              <div className="mt-2 flex h-2 rounded-full overflow-hidden bg-[#0B0D14]">
                {bars.map((value, index) => (
                  <span
                    key={`${item.id}-${index}`}
                    style={{ width: `${(value / sum) * 100}%`, background: blockFor(item.steps[index].stepType).color }}
                  />
                ))}
              </div>
              <div className="mt-1.5 text-[10.5px] text-[#707B94]">
                {item.steps.length} blocks · {formatMs(bars.reduce((acc, value) => acc + value, 0))}
              </div>
            </button>
          );
        })}
      </aside>

      <main
        className="overflow-y-auto p-5 flex flex-col gap-4"
        style={{
          backgroundImage: "radial-gradient(rgba(255,255,255,0.045) 1px, transparent 1px)",
          backgroundSize: "18px 18px",
          backgroundColor: "#0B0D14",
        }}
      >
        {macro && (
          <>
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1 min-w-0">
                <input
                  value={macro.name}
                  onChange={(event) => updateMacro((current) => ({ ...current, name: event.target.value, lastEditedMs: Date.now() }))}
                  className="w-full bg-transparent text-xl font-bold text-[#F4F5FA] outline-none"
                />
                <input
                  value={macro.description}
                  onChange={(event) =>
                    updateMacro((current) => ({ ...current, description: event.target.value, lastEditedMs: Date.now() }))
                  }
                  className="w-full mt-1 bg-transparent text-xs text-[#A6AFC4] outline-none"
                />
              </div>
              <button
                type="button"
                onClick={() => onRun(macro)}
                className="h-9 px-4 rounded-lg bg-[#35C98B] text-[#062117] text-xs font-bold flex items-center gap-1.5 cursor-pointer shrink-0"
              >
                <Play size={13} />
                Run chain
              </button>
            </div>

            <div className="flex items-center justify-between gap-3 text-xs text-[#A6AFC4]">
              <span>One chain at a time. Extra starts are rejected. Limit: 256 blocks and 60 seconds.</span>
              <button type="button" onClick={onStop} className="px-3 py-2 rounded border border-[#F16B79] text-[#F16B79] shrink-0">
                Stop running chain
              </button>
            </div>
            <div className="rounded-2xl border border-[#272D3D] bg-[#10131D]/90 p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-[11px] font-semibold text-[#A6AFC4]">Timing</span>
                <span className="text-[11px] font-mono text-[#F4F5FA]">{formatMs(total)}</span>
              </div>
              <svg viewBox="0 0 640 78" className="w-full h-[78px]">
                {(() => {
                  const gap = 8;
                  const usable = 640 - gap * Math.max(0, weights.length - 1);
                  const raw = weights.map((weight) => (weight / (total || 1)) * usable);
                  const grown = raw.reduce((sum, value) => sum + Math.max(value, 28), 0) || 1;
                  const widths = raw.map((value) => Math.max(28, value) * (usable / grown));
                  let cursor = 0;
                  return widths.map((width, index) => {
                    const x = cursor;
                    cursor += width + gap;
                    const kind = blockFor(macro.steps[index].stepType);
                    const selected = index === selectedStepIdx;
                    const actual = runReport?.macroId === macro.id
                      ? runReport.stepResults.find((result) => result.stepId === macro.steps[index].id)
                      : undefined;
                    const actualWidth = actual ? Math.max(8, Math.min(width, (actual.elapsedMs / (total || 1)) * usable)) : width * 0.35;
                    return (
                      <g key={macro.steps[index].id} onClick={() => onSelectStep(index)} className="cursor-pointer">
                        <rect x={x} y={8} width={width} height={28} rx={7} fill={kind.color} opacity={selected ? 1 : 0.55} />
                        <text x={x + 8} y={26} fill="#0B0D14" fontSize="11" fontWeight="700">
                          {width > 64 ? kind.title : String(index + 1)}
                        </text>
                        <rect
                          x={x}
                          y={44}
                          width={actualWidth}
                          height={8}
                          rx={4}
                          fill={actual ? (actual.status === "Passed" ? "#35C98B" : "#F16B79") : "#272D3D"}
                        />
                        <text x={x} y={70} fill="#707B94" fontSize="10">
                          {formatMs(actual?.elapsedMs ?? weights[index])}
                        </text>
                      </g>
                    );
                  });
                })()}
              </svg>
              <div className="text-[10.5px] text-[#707B94]">Top bars are the plan. Bottom bars are the last run.</div>
            </div>

            <div className="relative pl-8 flex flex-col gap-0">
              <div className="absolute left-[15px] top-3 bottom-3 w-px bg-gradient-to-b from-[#8B5CF6] via-[#56B9F2] to-[#35C98B]" />
              {macro.steps.length === 0 && (
                <div className="ml-4 rounded-2xl border border-dashed border-[#394158] p-8 text-center text-sm text-[#A6AFC4]">
                  Add the first block. The chain runs from top to bottom when the key is pressed.
                </div>
              )}
              {macro.steps.map((item, index) => {
                const kind = blockFor(item.stepType);
                const Glyph = kind.Icon;
                const selected = index === selectedStepIdx;
                return (
                  <div key={item.id} className="relative">
                    <button
                      type="button"
                      onClick={() => onSelectStep(index)}
                      className={`w-full text-left rounded-2xl border p-3 mb-3 cursor-pointer ${
                        selected ? "bg-[#1C2130] border-[#8B5CF6] shadow-[0_0_0_1px_#8B5CF6]" : "bg-[#151925] border-[#272D3D]"
                      }`}
                    >
                      <span
                        className="absolute -left-8 top-4 w-4 h-4 rounded-full border-2 border-[#0B0D14]"
                        style={{ background: kind.color }}
                      />
                      <div className="flex items-center gap-3">
                        <span className="w-8 h-8 rounded-lg flex items-center justify-center" style={{ background: `${kind.color}22` }}>
                          <Glyph size={16} color={kind.color} />
                        </span>
                        <div className="min-w-0 flex-1">
                          <div className="text-sm font-semibold text-[#F4F5FA]">{item.label || kind.title}</div>
                          <div className="text-[11px] font-mono text-[#A6AFC4] truncate">{item.parameter || kind.hint}</div>
                        </div>
                        <span className="text-[10.5px] font-mono text-[#707B94]">{formatMs(plannedMs(item))}</span>
                      </div>
                    </button>
                  </div>
                );
              })}
            </div>

            <div className="sticky bottom-0 rounded-2xl border border-[#272D3D] bg-[#10131D]/95 backdrop-blur p-2 flex flex-wrap gap-1.5">
              {BLOCKS.map((kind) => (
                <button
                  key={kind.type}
                  type="button"
                  onClick={() => addBlock(kind.type)}
                  className="h-8 px-2.5 rounded-lg border border-[#272D3D] bg-[#151925] hover:border-[#8B5CF6] text-[11.5px] font-semibold text-[#F4F5FA] flex items-center gap-1.5 cursor-pointer"
                >
                  <Plus size={12} color={kind.color} />
                  {kind.title}
                </button>
              ))}
            </div>
          </>
        )}
      </main>

      <aside className="bg-[#10131D] border-l border-[#272D3D] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="text-sm font-bold text-[#F4F5FA]">Block</div>
        {macro && step ? (
          <>
            <label className="text-[11px] text-[#A6AFC4]">Name</label>
            <input
              value={step.label}
              onChange={(event) => {
                const label = event.target.value;
                updateMacro((current) => ({
                  ...current,
                  steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, label } : item)),
                }));
              }}
              className="h-8 px-2.5 rounded-md bg-[#0B0D14] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
            />
            <label className="text-[11px] text-[#A6AFC4]">Kind</label>
            <select
              value={step.stepType}
              onChange={(event) => {
                const stepType = event.target.value as StepType;
                const kind = blockFor(stepType);
                updateMacro((current) => ({
                  ...current,
                  steps: current.steps.map((item, index) =>
                    index === selectedStepIdx ? { ...item, stepType, label: kind.title, parameter: kind.parameter } : item
                  ),
                }));
              }}
              className="h-8 px-2 rounded-md bg-[#0B0D14] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
            >
              {BLOCKS.map((kind) => (
                <option key={kind.type} value={kind.type}>
                  {kind.title}
                </option>
              ))}
            </select>
            <label className="text-[11px] text-[#A6AFC4]">{blockFor(step.stepType).hint}</label>
            {step.stepType === "SwitchProfile" ? (
              <select
                value={step.parameter}
                onChange={(event) => {
                  const parameter = event.target.value;
                  updateMacro((current) => ({
                    ...current,
                    steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, parameter } : item)),
                  }));
                }}
                className="h-8 px-2 rounded-md bg-[#0B0D14] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
              >
                {profiles.map((profile) => (
                  <option key={profile.id} value={profile.id}>
                    {profile.name}
                  </option>
                ))}
              </select>
            ) : (
              <input
                value={step.parameter}
                onChange={(event) => {
                  const parameter = event.target.value;
                  updateMacro((current) => ({
                    ...current,
                    steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, parameter } : item)),
                  }));
                }}
                className="h-8 px-2.5 rounded-md bg-[#0B0D14] border border-[#272D3D] text-xs font-mono text-[#F4F5FA] outline-none"
              />
            )}
            <label className="text-[11px] text-[#A6AFC4]">Block timeout (ms, including retry)</label>
            <input type="number" min={1} max={60000} value={step.timeoutMs}
              onChange={(event) => {
                const timeoutMs = Math.max(1, Math.min(60000, Number(event.target.value) || 1));
                updateMacro(current => ({ ...current, steps: current.steps.map((item, index) =>
                  index === selectedStepIdx ? { ...item, timeoutMs } : item) }));
              }}
              className="h-8 px-2.5 rounded-md bg-[#0B0D14] border border-[#272D3D] text-xs" />
            <p className="text-[10px] text-[#707B94]">Timeout or Stop ends the chain. An action already dispatched cannot be undone.</p>
            <label className="text-[11px] text-[#A6AFC4]">If this block fails</label>
            <select
              value={step.onError}
              onChange={(event) => {
                const onError = event.target.value as MacroStep["onError"];
                updateMacro((current) => ({
                  ...current,
                  steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, onError } : item)),
                }));
              }}
              className="h-8 px-2 rounded-md bg-[#0B0D14] border border-[#272D3D] text-xs text-[#F4F5FA] outline-none"
            >
              <option value="Stop">Stop the chain</option>
              <option value="Continue">Keep going</option>
              <option value="RetryOnce">Try once more</option>
            </select>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => {
                  if (selectedStepIdx === 0) return;
                  updateMacro((current) => {
                    const steps = [...current.steps];
                    const [moved] = steps.splice(selectedStepIdx, 1);
                    steps.splice(selectedStepIdx - 1, 0, moved);
                    return { ...current, steps };
                  });
                  onSelectStep(selectedStepIdx - 1);
                }}
                className="h-8 flex-1 rounded-md border border-[#272D3D] text-[#A6AFC4] flex items-center justify-center cursor-pointer"
              >
                <ChevronUp size={14} />
              </button>
              <button
                type="button"
                onClick={() => {
                  if (!macro || selectedStepIdx >= macro.steps.length - 1) return;
                  updateMacro((current) => {
                    const steps = [...current.steps];
                    const [moved] = steps.splice(selectedStepIdx, 1);
                    steps.splice(selectedStepIdx + 1, 0, moved);
                    return { ...current, steps };
                  });
                  onSelectStep(selectedStepIdx + 1);
                }}
                className="h-8 flex-1 rounded-md border border-[#272D3D] text-[#A6AFC4] flex items-center justify-center cursor-pointer"
              >
                <ChevronDown size={14} />
              </button>
              <button
                type="button"
                onClick={() => {
                  updateMacro((current) => ({
                    ...current,
                    steps: current.steps.filter((_, index) => index !== selectedStepIdx),
                  }));
                  onSelectStep(Math.max(0, selectedStepIdx - 1));
                }}
                className="h-8 w-8 rounded-md border border-[#3A2430] text-[#F16B79] flex items-center justify-center cursor-pointer"
              >
                <Trash2 size={14} />
              </button>
            </div>
          </>
        ) : (
          <p className="text-xs text-[#A6AFC4]">Select a block in the chain, or add one from the row below the graph.</p>
        )}

        {macro && (
          <button
            type="button"
            onClick={() => onAssign(macro)}
            className="mt-auto h-10 rounded-lg bg-[#30244F] border border-[#8B5CF6] text-[#F4F5FA] text-xs font-semibold flex items-center justify-center gap-1.5 cursor-pointer"
          >
            <Zap size={14} />
            {activeKeyLabel ? `Put on ${activeKeyLabel}` : "Put on selected key"}
          </button>
        )}
        {runReport && runReport.macroId === macro?.id && (
          <div className="rounded-xl border border-[#272D3D] bg-[#0B0D14] p-2.5 text-[11px] font-mono text-[#A6AFC4]">
            {runReport.stepResults.map((result) => (
              <div key={result.stepId} className="py-0.5">
                <span className={result.status === "Passed" ? "text-[#35C98B]" : "text-[#F16B79]"}>{result.status}</span>{" "}
                {result.detail}
              </div>
            ))}
          </div>
        )}
      </aside>
    </div>
  );
};
