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

  const blockNameId = React.useId();
  const blockKindId = React.useId();
  const blockParamId = React.useId();
  const blockTimeoutId = React.useId();
  const blockErrorId = React.useId();

  return (
    <div className="flex-1 min-h-0 grid grid-cols-[232px_minmax(0,1fr)_284px] 2xl:grid-cols-[256px_minmax(0,1fr)_312px] overflow-hidden animate-screen-in">
      <aside className="min-h-0 bg-[var(--bg-sidebar)] border-r border-[var(--border-subtle)] p-3 flex flex-col gap-2 overflow-y-auto">
        <button
          type="button"
          onClick={createMacro}
          className="h-9 rounded-lg bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white text-xs font-semibold flex items-center justify-center gap-1.5 cursor-pointer shadow-xs active:scale-[0.98] transition-all"
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
              aria-pressed={active}
              onClick={() => {
                onSelectMacro(item.id);
                onSelectStep(0);
              }}
              className={`text-left p-3 rounded-xl border cursor-pointer transition-all ${
                active ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/70 shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]" : "bg-[var(--bg-panel)] border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-elevated)] hover:-translate-y-[0.5px]"
              }`}
            >
              <div className="text-xs font-bold text-[var(--text-primary)] truncate">{item.name}</div>
              <div className="mt-2 flex h-2 rounded-full overflow-hidden bg-[var(--bg-app)]">
                {bars.map((value, index) => (
                  <span
                    key={`${item.id}-${index}`}
                    style={{ width: `${(value / sum) * 100}%`, background: blockFor(item.steps[index].stepType).color }}
                  />
                ))}
              </div>
              <div className="mt-1.5 text-[10.5px] text-[var(--text-muted)]">
                {item.steps.length} blocks · {formatMs(bars.reduce((acc, value) => acc + value, 0))}
              </div>
            </button>
          );
        })}
      </aside>

      <main className="min-h-0 overflow-y-auto p-5 flex flex-col gap-4 df-dot-grid">
        {macro && (
          <>
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1 min-w-0">
                <input
                  aria-label="Macro chain name"
                  value={macro.name}
                  onChange={(event) => updateMacro((current) => ({ ...current, name: event.target.value, lastEditedMs: Date.now() }))}
                  className="w-full bg-transparent hover:bg-[var(--bg-panel)]/50 focus:bg-[var(--bg-panel)] rounded-md px-2 -mx-2 py-0.5 border border-transparent focus:border-[var(--border-subtle)] text-xl font-bold text-[var(--text-primary)] outline-none"
                />
                <input
                  aria-label="Macro chain description"
                  value={macro.description}
                  onChange={(event) =>
                    updateMacro((current) => ({ ...current, description: event.target.value, lastEditedMs: Date.now() }))
                  }
                  className="w-full mt-1 bg-transparent hover:bg-[var(--bg-panel)]/50 focus:bg-[var(--bg-panel)] rounded-md px-2 -mx-2 py-0.5 border border-transparent focus:border-[var(--border-subtle)] text-xs text-[var(--text-secondary)] outline-none"
                />
              </div>
              <button
                type="button"
                onClick={() => onRun(macro)}
                className="h-9 px-4 rounded-lg bg-[var(--status-success)] hover:brightness-110 active:scale-[0.98] text-[#062117] text-xs font-bold flex items-center gap-1.5 cursor-pointer shrink-0 transition-all"
              >
                <Play size={13} />
                Run chain
              </button>
            </div>

            <div className="flex items-center justify-between gap-3 text-xs text-[var(--text-secondary)]">
              <span>One chain at a time. Extra starts are rejected. Limit: 256 blocks and 60 seconds.</span>
              <button type="button" onClick={onStop} className="h-8 px-3 rounded-md border border-[var(--status-error)]/60 hover:bg-[var(--status-error)]/10 text-[var(--status-error)] font-medium cursor-pointer shrink-0 transition-colors">
                Stop running chain
              </button>
            </div>
            <div className="rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-sidebar)]/90 p-3.5">
              <div className="flex items-center justify-between mb-2">
                <span className="text-[11px] font-semibold text-[var(--text-secondary)]">Timing</span>
                <span className="text-[11px] font-mono tabular-nums text-[var(--text-primary)]">{formatMs(total)}</span>
              </div>
              {macro.steps.length === 0 ? (
                <div className="py-3 text-center text-[11.5px] text-[var(--text-muted)]">
                  Add blocks below to preview the execution timeline.
                </div>
              ) : (
                <>
                  <svg viewBox="0 0 640 78" className="w-full h-[78px]" aria-label="Macro execution timeline">
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
                          <g
                            key={macro.steps[index].id}
                            role="button"
                            tabIndex={0}
                            aria-label={`Block ${index + 1}: ${kind.title}`}
                            onClick={() => onSelectStep(index)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ") {
                                e.preventDefault();
                                onSelectStep(index);
                              }
                            }}
                            className="cursor-pointer hover:opacity-90 transition-opacity"
                          >
                            <rect x={x} y={8} width={width} height={28} rx={6} fill={kind.color} fillOpacity={selected ? 0.3 : 0.14} stroke={kind.color} strokeOpacity={selected ? 0.9 : 0} />
                            <rect x={x} y={8} width={3} height={28} rx={1.5} fill={kind.color} />
                            <text x={x + 10} y={26} fill={kind.color} fontSize="11" fontWeight="600">
                              {width > 64 ? kind.title : String(index + 1)}
                            </text>
                            <rect x={x} y={44} width={width} height={8} rx={4} fill="var(--bg-app)" />
                            <rect
                              x={x}
                              y={44}
                              width={actualWidth}
                              height={8}
                              rx={4}
                              fill={actual ? (actual.status === "Passed" ? "var(--status-success)" : "var(--status-error)") : "var(--border-strong)"}
                            />
                            <text x={x} y={70} fill="var(--text-muted)" fontSize="10">
                              {formatMs(actual?.elapsedMs ?? weights[index])}
                            </text>
                          </g>
                        );
                      });
                    })()}
                  </svg>
                  <div className="text-[10.5px] text-[var(--text-muted)]">Top bars are the plan. Bottom bars are the last run.</div>
                </>
              )}
            </div>

            <div className="relative pl-8 flex flex-col gap-0">
              <div className="absolute left-[15px] top-3 bottom-3 w-px bg-[var(--border-strong)]" />
              {macro.steps.length === 0 && (
                <div className="ml-4 rounded-2xl border border-dashed border-[var(--border-strong)] p-8 text-center text-sm text-[var(--text-secondary)]">
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
                      aria-pressed={selected}
                      onClick={() => onSelectStep(index)}
                      className={`w-full text-left rounded-2xl border p-3 mb-3 cursor-pointer transition-all ${
                        selected ? "bg-[var(--bg-elevated)] border-[var(--accent-primary)] ring-1 ring-[var(--accent-primary)] shadow-md" : "bg-[var(--bg-panel)] border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:-translate-y-[0.5px]"
                      }`}
                    >
                      <span
                        className={`absolute -left-8 top-4 w-4 h-4 rounded-full border-2 border-[var(--bg-app)] transition-transform ${selected ? "scale-110 ring-2 ring-[var(--accent-primary)]/40" : ""}`}
                        style={{ background: kind.color }}
                      />
                      <div className="flex items-center gap-3">
                        <span className="w-8 h-8 rounded-lg flex items-center justify-center" style={{ background: `${kind.color}22` }}>
                          <Glyph size={16} color={kind.color} />
                        </span>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span className="text-[10.5px] font-mono tabular-nums text-[var(--text-muted)]">#{index + 1}</span>
                            <span className="text-sm font-semibold text-[var(--text-primary)] truncate">{item.label || kind.title}</span>
                          </div>
                          <div className="text-[11px] font-mono text-[var(--text-secondary)] truncate">{item.parameter || kind.hint}</div>
                        </div>
                        <span className="text-[10.5px] font-mono tabular-nums text-[var(--text-muted)]">{formatMs(plannedMs(item))}</span>
                      </div>
                    </button>
                  </div>
                );
              })}
            </div>

            <div className="sticky bottom-0 mt-auto rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-sidebar)]/95 backdrop-blur-md p-2 shadow-xl flex flex-wrap gap-1.5">
              {BLOCKS.map((kind) => (
                <button
                  key={kind.type}
                  type="button"
                  onClick={() => addBlock(kind.type)}
                  className="h-8 px-2.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:border-[var(--accent-primary)] hover:bg-[var(--bg-elevated)] hover:-translate-y-[1px] text-[11.5px] font-semibold text-[var(--text-primary)] flex items-center gap-1.5 cursor-pointer transition-all"
                >
                  <Plus size={12} color={kind.color} />
                  {kind.title}
                </button>
              ))}
            </div>
          </>
        )}
      </main>

      <aside className="min-h-0 bg-[var(--bg-sidebar)] border-l border-[var(--border-subtle)] p-4 flex flex-col gap-3 overflow-y-auto">
        <div className="flex items-center justify-between pb-2.5 border-b border-[var(--border-subtle)]">
          <div className="text-sm font-bold text-[var(--text-primary)]">Block</div>
          {macro && step && (
            <span className="text-[10.5px] font-mono px-2 py-0.5 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)]">
              #{selectedStepIdx + 1} · {blockFor(step.stepType).title}
            </span>
          )}
        </div>
        {macro && step ? (
          <>
            <div className="flex flex-col gap-1">
              <label htmlFor={blockNameId} className="text-[11px] text-[var(--text-secondary)]">Name</label>
              <input
                id={blockNameId}
                value={step.label}
                onChange={(event) => {
                  const label = event.target.value;
                  updateMacro((current) => ({
                    ...current,
                    steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, label } : item)),
                  }));
                }}
                className="h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none transition-colors"
              />
            </div>
            <div className="flex flex-col gap-1">
              <label htmlFor={blockKindId} className="text-[11px] text-[var(--text-secondary)]">Kind</label>
              <select
                id={blockKindId}
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
                className="h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer transition-colors"
              >
                {BLOCKS.map((kind) => (
                  <option key={kind.type} value={kind.type}>
                    {kind.title}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label htmlFor={blockParamId} className="text-[11px] text-[var(--text-secondary)]">{blockFor(step.stepType).hint}</label>
              {step.stepType === "SwitchProfile" ? (
                <select
                  id={blockParamId}
                  value={step.parameter}
                  onChange={(event) => {
                    const parameter = event.target.value;
                    updateMacro((current) => ({
                      ...current,
                      steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, parameter } : item)),
                    }));
                  }}
                  className="h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer transition-colors"
                >
                  {profiles.map((profile) => (
                    <option key={profile.id} value={profile.id}>
                      {profile.name}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  id={blockParamId}
                  value={step.parameter}
                  onChange={(event) => {
                    const parameter = event.target.value;
                    updateMacro((current) => ({
                      ...current,
                      steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, parameter } : item)),
                    }));
                  }}
                  className="h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs font-mono text-[var(--text-primary)] outline-none transition-colors"
                />
              )}
            </div>
            <div className="flex flex-col gap-1">
              <label htmlFor={blockTimeoutId} className="text-[11px] text-[var(--text-secondary)]">Block timeout (ms, including retry)</label>
              <input id={blockTimeoutId} type="number" min={1} max={60000} value={step.timeoutMs}
                onChange={(event) => {
                  const timeoutMs = Math.max(1, Math.min(60000, Number(event.target.value) || 1));
                  updateMacro(current => ({ ...current, steps: current.steps.map((item, index) =>
                    index === selectedStepIdx ? { ...item, timeoutMs } : item) }));
                }}
                className="h-8 px-2.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs font-mono tabular-nums text-[var(--text-primary)] outline-none transition-colors" />
              <p className="text-[10px] text-[var(--text-muted)] leading-snug">Timeout or Stop ends the chain. An action already dispatched cannot be undone.</p>
            </div>
            <div className="flex flex-col gap-1">
              <label htmlFor={blockErrorId} className="text-[11px] text-[var(--text-secondary)]">If this block fails</label>
              <select
                id={blockErrorId}
                value={step.onError}
                onChange={(event) => {
                  const onError = event.target.value as MacroStep["onError"];
                  updateMacro((current) => ({
                    ...current,
                    steps: current.steps.map((item, index) => (index === selectedStepIdx ? { ...item, onError } : item)),
                  }));
                }}
                className="h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none cursor-pointer"
              >
                <option value="Stop">Stop the chain</option>
                <option value="Continue">Keep going</option>
                <option value="RetryOnce">Try once more</option>
              </select>
            </div>
            <div className="flex gap-2">
              <button
                type="button"
                disabled={selectedStepIdx === 0}
                title="Move block up"
                aria-label="Move block up"
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
                className="h-8 flex-1 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.97] text-[var(--text-secondary)] hover:text-[var(--text-primary)] disabled:opacity-35 disabled:cursor-default flex items-center justify-center cursor-pointer transition-all"
              >
                <ChevronUp size={14} />
              </button>
              <button
                type="button"
                disabled={!macro || selectedStepIdx >= macro.steps.length - 1}
                title="Move block down"
                aria-label="Move block down"
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
                className="h-8 flex-1 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-panel)] hover:bg-[var(--bg-elevated)] hover:border-[var(--border-strong)] active:scale-[0.97] text-[var(--text-secondary)] hover:text-[var(--text-primary)] disabled:opacity-35 disabled:cursor-default flex items-center justify-center cursor-pointer transition-all"
              >
                <ChevronDown size={14} />
              </button>
              <button
                type="button"
                title="Remove block"
                aria-label="Remove block"
                onClick={() => {
                  updateMacro((current) => ({
                    ...current,
                    steps: current.steps.filter((_, index) => index !== selectedStepIdx),
                  }));
                  onSelectStep(Math.max(0, selectedStepIdx - 1));
                }}
                className="h-8 w-8 rounded-md border border-[var(--status-error)]/35 hover:border-[var(--status-error)] hover:bg-[var(--status-error)]/10 active:scale-[0.97] text-[var(--status-error)] flex items-center justify-center cursor-pointer transition-all"
              >
                <Trash2 size={14} />
              </button>
            </div>
          </>
        ) : (
          <p className="text-xs text-[var(--text-secondary)]">Select a block in the chain, or add one from the row below the graph.</p>
        )}

        {macro && (
          <button
            type="button"
            onClick={() => onAssign(macro)}
            className="mt-auto h-10 rounded-lg bg-[var(--accent-soft)] border border-[var(--accent-primary)]/60 hover:bg-[var(--accent-primary)] active:scale-[0.98] text-[var(--accent-text)] hover:text-white text-xs font-semibold flex items-center justify-center gap-1.5 cursor-pointer transition-all"
          >
            <Zap size={14} />
            {activeKeyLabel ? `Put on ${activeKeyLabel}` : "Put on selected key"}
          </button>
        )}
        {runReport && runReport.macroId === macro?.id && (
          <div className="rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-app)] p-2.5 text-[11px] font-mono text-[var(--text-secondary)] flex flex-col gap-1">
            {runReport.stepResults.map((result) => (
              <div key={result.stepId} className="flex items-start gap-2 py-0.5">
                <span
                  className={`shrink-0 px-1.5 py-0.5 rounded text-[9.5px] font-semibold ${
                    result.status === "Passed"
                      ? "bg-[var(--status-success)]/15 text-[var(--status-success)]"
                      : "bg-[var(--status-error)]/15 text-[var(--status-error)]"
                  }`}
                >
                  {result.status}
                </span>
                <span className="min-w-0 break-words leading-snug">{result.detail}</span>
              </div>
            ))}
          </div>
        )}
      </aside>
    </div>
  );
};
