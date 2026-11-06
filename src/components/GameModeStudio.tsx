import React from "react";
import {
  Bot,
  Check,
  Gamepad2,
  Lock,
  MemoryStick,
  Plus,
  Power,
  RefreshCw,
  RotateCcw,
  Search,
  Server,
  ShieldCheck,
  Star,
  Trash2,
  TriangleAlert,
  X,
  Zap,
} from "lucide-react";
import { deckforgeApi } from "../bridge/tauriBridge";
import type {
  GameModeConfig,
  GameModeCustomTarget,
  GameModeGroup,
  GameModeMode,
  GameModeScan,
  GameModeStatus,
  GameModeTarget,
} from "../types/deckforge";

type Notify = (text: string, kind?: "success" | "info" | "warning") => void;

const GROUPS: { id: GameModeGroup; title: string; blurb: string; Icon: React.ComponentType<{ size?: number }> }[] = [
  { id: "apps", title: "Apps", blurb: "Programs Game Mode closes. Untick anything you want to keep open.", Icon: MemoryStick },
  { id: "ai", title: "AI apps", blurb: "Claude, Codex, ChatGPT and friends, plus their helper services.", Icon: Bot },
  { id: "background", title: "Background services and tray apps", blurb: "Services stop through the admin helper. Tray apps close like any other app.", Icon: Server },
];

const RESTORE_OPTIONS: { id: GameModeConfig["restore"]; title: string; hint: string }[] = [
  { id: "all", title: "Put everything back", hint: "Reopen closed apps one by one and restart services" },
  { id: "settings", title: "Services only", hint: "Restart services, leave apps closed" },
  { id: "none", title: "Leave it", hint: "Nothing is restored until you do it" },
];

const ELEVATE_OPTIONS: { id: GameModeConfig["elevate"]; title: string; hint: string }[] = [
  { id: "ask", title: "Use the admin helper", hint: "Stops services, restores them when Game Mode ends" },
  { id: "skip", title: "Never stop services", hint: "Only close apps and apply tweaks" },
];

const SETTLE_OPTIONS = [10, 20, 40, 60];

function formatMemory(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
  return `${mb} MB`;
}

function formatElapsed(startedAtMs: number, now: number): string {
  const seconds = Math.max(0, Math.floor((now - startedAtMs) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

const slug = (text: string) =>
  text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 24) || "custom";

const Checkbox: React.FC<{ checked: boolean }> = ({ checked }) => (
  <span
    aria-hidden="true"
    className={`w-4 h-4 rounded-[4px] border flex items-center justify-center shrink-0 ${
      checked ? "bg-[#8B5CF6] border-[#8B5CF6] text-white" : "bg-[#0E111A] border-[#3A4157] text-transparent"
    }`}
  >
    <Check size={11} strokeWidth={3} />
  </span>
);

const Stat: React.FC<{ label: string; value: string; hint?: string; tone?: "good" | "warn" }> = ({ label, value, hint, tone }) => (
  <div className="bg-[#10131D] border border-[#272D3D] rounded-lg px-3.5 py-3 min-w-0">
    <div className="text-[10.5px] uppercase tracking-wider text-[#707B94]">{label}</div>
    <div className={`mt-1 text-[15px] font-semibold truncate ${tone === "good" ? "text-[#35C98B]" : tone === "warn" ? "text-[#F4B860]" : "text-[#F4F5FA]"}`}>
      {value}
    </div>
    {hint && <div className="text-[11px] text-[#707B94] mt-0.5 truncate">{hint}</div>}
  </div>
);

const Toggle: React.FC<{ on: boolean; onClick: () => void; title: string; hint: string; label: string }> = ({ on, onClick, title, hint, label }) => (
  <button
    onClick={onClick}
    aria-label={`${label}${on ? ", on" : ", off"}`}
    className={`text-left flex items-start gap-3 px-3.5 py-2.5 rounded-lg border cursor-pointer transition-colors ${
      on ? "bg-[#151126] border-[#8B5CF6]/45" : "bg-[#10131D] border-[#272D3D] hover:border-[#3A4157]"
    }`}
  >
    <span className="mt-0.5">
      <Checkbox checked={on} />
    </span>
    <span className="min-w-0">
      <span className="block text-[12.5px] font-semibold text-[#F4F5FA]">{title}</span>
      <span className="block text-[11.5px] text-[#A6AFC4] mt-0.5 leading-snug">{hint}</span>
    </span>
  </button>
);

export const GameModeStudio: React.FC<{ notify: Notify }> = ({ notify }) => {
  const [scan, setScan] = React.useState<GameModeScan | null>(null);
  const [config, setConfig] = React.useState<GameModeConfig | null>(null);
  const [status, setStatus] = React.useState<GameModeStatus | null>(null);
  const [scanning, setScanning] = React.useState(false);
  const [savedAt, setSavedAt] = React.useState<number>(0);
  const [now, setNow] = React.useState<number>(() => Date.now());
  const [editingId, setEditingId] = React.useState<string>("");
  const [showIdle, setShowIdle] = React.useState(false);
  const [confirmDelete, setConfirmDelete] = React.useState(false);
  const [arming, setArming] = React.useState(false);

  // Picker for the user's own targets.
  const [pickerOpen, setPickerOpen] = React.useState(false);
  const [pickerTab, setPickerTab] = React.useState<"programs" | "services">("programs");
  const [query, setQuery] = React.useState("");
  const [pickedProcs, setPickedProcs] = React.useState<string[]>([]);
  const [pickedServices, setPickedServices] = React.useState<string[]>([]);
  const [customLabel, setCustomLabel] = React.useState("");
  const [customGroup, setCustomGroup] = React.useState<GameModeCustomTarget["group"]>("apps");
  const [customGraceful, setCustomGraceful] = React.useState(true);
  const [customReopen, setCustomReopen] = React.useState(true);

  const refresh = React.useCallback(async () => {
    setScanning(true);
    try {
      const next = await deckforgeApi.gameModeScan();
      setScan(next);
      return next;
    } finally {
      setScanning(false);
    }
  }, []);

  React.useEffect(() => {
    let cancelled = false;
    void (async () => {
      const [cfg, st] = await Promise.all([deckforgeApi.gameModeGetConfig(), deckforgeApi.gameModeStatus()]);
      if (cancelled) return;
      setConfig(cfg);
      setStatus(st);
      setEditingId((current) => current || cfg.activeMode);
      await refresh();
    })();
    let unlisten: (() => void) | null = null;
    void deckforgeApi
      .onGameMode((next) => {
        setStatus(next);
        if (!next.busy) void refresh();
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    const tick = window.setInterval(() => setNow(Date.now()), 15000);
    return () => {
      cancelled = true;
      unlisten?.();
      window.clearInterval(tick);
    };
  }, [refresh]);

  const save = async (next: GameModeConfig) => {
    setConfig(next);
    const saved = await deckforgeApi.gameModeSaveConfig(next);
    setConfig(saved);
    setSavedAt(Date.now());
    return saved;
  };

  const mode: GameModeMode | null = config ? config.modes.find((m) => m.id === editingId) ?? config.modes[0] : null;

  const updateMode = (patch: Partial<GameModeMode>) => {
    if (!config || !mode) return;
    void save({ ...config, modes: config.modes.map((m) => (m.id === mode.id ? { ...m, ...patch } : m)) });
  };

  const toggleTarget = (id: string) => {
    if (!mode) return;
    const on = mode.targets.includes(id);
    updateMode({ targets: on ? mode.targets.filter((t) => t !== id) : [...mode.targets, id] });
  };

  const setGroup = (ids: string[], on: boolean) => {
    if (!mode) return;
    const rest = mode.targets.filter((id) => !ids.includes(id));
    updateMode({ targets: on ? [...rest, ...ids] : rest });
  };

  const toggleTweak = (id: string) => {
    if (!mode) return;
    const on = mode.tweaks.includes(id);
    updateMode({ tweaks: on ? mode.tweaks.filter((t) => t !== id) : [...mode.tweaks, id] });
  };

  const addMode = () => {
    if (!config || !mode) return;
    let n = config.modes.length + 1;
    while (config.modes.some((m) => m.id === `mode-${n}`)) n += 1;
    const created: GameModeMode = { id: `mode-${n}`, name: `Mode ${n}`, targets: [...mode.targets], tweaks: [...mode.tweaks] };
    setEditingId(created.id);
    setConfirmDelete(false);
    void save({ ...config, modes: [...config.modes, created] });
  };

  const renameMode = (name: string) => {
    if (!mode || name.trim() === "" || name.trim() === mode.name) return;
    updateMode({ name: name.trim() });
  };

  const deleteMode = () => {
    if (!config || !mode || config.modes.length < 2) return;
    if (!confirmDelete) {
      setConfirmDelete(true);
      return;
    }
    const modes = config.modes.filter((m) => m.id !== mode.id);
    setEditingId(modes[0].id);
    setConfirmDelete(false);
    void save({ ...config, modes, activeMode: config.activeMode === mode.id ? modes[0].id : config.activeMode });
  };

  const resetPicker = () => {
    setPickerOpen(false);
    setQuery("");
    setPickedProcs([]);
    setPickedServices([]);
    setCustomLabel("");
    setCustomGroup("apps");
    setCustomGraceful(true);
    setCustomReopen(true);
  };

  const addCustom = async () => {
    if (!config || !mode) return;
    if (pickedProcs.length === 0 && pickedServices.length === 0) return;
    const label = customLabel.trim() || pickedProcs[0] || pickedServices[0];
    const taken = new Set([...config.customTargets.map((c) => c.id), ...(scan?.targets.map((t) => t.id) ?? [])]);
    let id = `my-${slug(label)}`;
    let n = 2;
    while (taken.has(id)) id = `my-${slug(label)}-${n++}`;
    const target: GameModeCustomTarget = {
      id,
      label,
      group: pickedProcs.length === 0 ? "background" : customGroup,
      processes: pickedProcs,
      services: pickedServices,
      graceful: customGraceful,
      reopen: pickedProcs.length > 0 && customReopen,
    };
    resetPicker();
    await save({
      ...config,
      customTargets: [...config.customTargets, target],
      modes: config.modes.map((m) => (m.id === mode.id ? { ...m, targets: [...m.targets, id] } : m)),
    });
    await refresh();
    notify(`Added ${label} to ${mode.name}`, "success");
  };

  const removeCustom = async (id: string) => {
    if (!config) return;
    await save({
      ...config,
      customTargets: config.customTargets.filter((c) => c.id !== id),
      modes: config.modes.map((m) => ({ ...m, targets: m.targets.filter((t) => t !== id) })),
    });
    await refresh();
  };

  const active = status?.active ?? false;
  const busy = status?.busy ?? null;

  const toggleGameMode = async () => {
    if (busy) return;
    try {
      await deckforgeApi.gameModeSet(!active, active ? undefined : mode?.id);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error), "warning");
    }
  };

  const armHelper = async () => {
    if (arming) return;
    setArming(true);
    try {
      await deckforgeApi.gameModeArmHelper();
      notify("Windows will ask once. Approve it to start the admin helper.", "info");
      // The prompt takes as long as the user does, so watch for the helper to appear.
      const until = Date.now() + 100000;
      while (Date.now() < until) {
        await new Promise((resolve) => window.setTimeout(resolve, 1500));
        const next = await refresh();
        if (next.helper.armed && next.helper.covers) {
          notify("Admin helper is running.", "success");
          break;
        }
      }
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error), "warning");
    } finally {
      setArming(false);
    }
  };

  const picked = React.useMemo(() => {
    if (!scan || !mode) return { count: 0, memoryMb: 0, services: 0 };
    const chosen = scan.targets.filter((t) => mode.targets.includes(t.id));
    return {
      count: chosen.filter((t) => t.running).length,
      memoryMb: chosen.reduce((sum, t) => sum + t.memoryMb, 0),
      services: chosen.reduce((sum, t) => sum + t.services.filter((s) => s.running).length, 0),
    };
  }, [scan, mode]);

  const rowFor = (target: GameModeTarget) => {
    const on = mode?.targets.includes(target.id) ?? false;
    const runningServices = target.services.filter((s) => s.running);
    return (
      <div key={target.id} className="relative">
        <button
          onClick={() => toggleTarget(target.id)}
          disabled={!mode}
          aria-label={`${target.label}${on ? ", closes in this mode" : ", stays open"}`}
          className={`w-full h-full text-left flex items-start gap-3 px-3.5 py-2.5 rounded-lg border transition-colors cursor-pointer ${
            on ? "bg-[#151126] border-[#8B5CF6]/45" : "bg-[#10131D] border-[#272D3D] hover:border-[#3A4157]"
          } ${target.custom ? "pr-10" : ""}`}
        >
          <span className="mt-0.5">
            <Checkbox checked={on} />
          </span>
          <span className="flex-1 min-w-0">
            <span className="flex items-center gap-2 flex-wrap">
              <span className="text-[12.5px] font-semibold text-[#F4F5FA]">{target.label}</span>
              {target.custom && (
                <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#30244F] text-[#B79CFF] border border-[#8B5CF6]/40">Yours</span>
              )}
              {target.running ? (
                <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#35C98B]/12 text-[#35C98B] border border-[#35C98B]/30">
                  {target.processCount > 0
                    ? `Running · ${formatMemory(target.memoryMb)}${target.processCount > 1 ? ` · ${target.processCount} processes` : ""}`
                    : `Service running`}
                </span>
              ) : (
                <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#1A1F2E] text-[#707B94] border border-[#272D3D]">Not running</span>
              )}
              {target.processCount > 0 && runningServices.length > 0 && (
                <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#1A1F2E] text-[#A6AFC4] border border-[#272D3D]">
                  + {runningServices.length} service{runningServices.length > 1 ? "s" : ""}
                </span>
              )}
              {target.needsAdmin && (
                <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#F4B860]/10 text-[#F4B860] border border-[#F4B860]/30 flex items-center gap-1">
                  <Lock size={9} /> helper
                </span>
              )}
            </span>
            <span className="block text-[11.5px] text-[#A6AFC4] mt-0.5 leading-snug">
              {target.custom
                ? [target.processes.length ? `Programs: ${target.processes.join(", ")}` : "", target.services.length ? `Services: ${target.services.map((s) => s.name).join(", ")}` : ""]
                    .filter(Boolean)
                    .join(" · ")
                : target.note}
            </span>
          </span>
        </button>
        {target.custom && (
          <button
            onClick={() => void removeCustom(target.id)}
            aria-label={`Remove ${target.label}`}
            title={`Remove ${target.label} from every mode`}
            className="absolute top-2 right-2 w-6 h-6 rounded-md flex items-center justify-center text-[#707B94] hover:text-[#F16B79] hover:bg-[#F16B79]/10 cursor-pointer"
          >
            <Trash2 size={13} />
          </button>
        )}
      </div>
    );
  };

  if (!scan || !config || !mode) {
    return (
      <div className="flex-1 flex items-center justify-center text-sm text-[#A6AFC4]">Analysing your PC...</div>
    );
  }

  const bitsumOk = scan.bitsumFound;
  const freedMb = status ? Math.max(0, status.ramFreeAfterMb - status.ramFreeBeforeMb) : 0;
  const q = query.trim().toLowerCase();
  const programRows = scan.processes.filter((p) => !q || p.name.includes(q));
  const serviceRows = scan.services.filter((s) => !q || s.name.toLowerCase().includes(q) || s.display.toLowerCase().includes(q));
  const selectedHasServices = scan.targets.some((t) => mode.targets.includes(t.id) && t.needsAdmin);
  const idleCount = scan.targets.filter((t) => !(t.running || t.custom || mode.targets.includes(t.id))).length;
  const helper = scan.helper;

  return (
    <div className="flex-1 grid grid-cols-[1fr_340px] overflow-hidden">
      <div className="overflow-y-auto p-6 flex flex-col gap-5">
        {/* Header and master switch */}
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-start gap-3">
            <div className={`w-11 h-11 rounded-xl flex items-center justify-center ${active ? "bg-[#35C98B]/15 text-[#35C98B]" : "bg-[#30244F] text-[#B79CFF]"}`}>
              <Gamepad2 size={22} />
            </div>
            <div>
              <h1 className="text-xl font-bold text-[#F4F5FA]">Game Mode</h1>
              <p className="text-xs text-[#A6AFC4] mt-0.5 max-w-xl leading-relaxed">
                Build your own modes: pick what to close and which services to stop. Press the Game Mode key on the deck, or let DeckForge start
                one when a game goes full-screen. Everything is put back when you are done.
              </p>
            </div>
          </div>
          <button
            onClick={() => void toggleGameMode()}
            disabled={Boolean(busy)}
            className={`h-11 px-5 rounded-lg text-sm font-semibold flex items-center gap-2 shrink-0 transition-colors cursor-pointer disabled:cursor-wait ${
              busy
                ? "bg-[#1C2130] text-[#A6AFC4] border border-[#272D3D]"
                : active
                  ? "bg-[#F16B79]/15 text-[#F16B79] border border-[#F16B79]/50 hover:bg-[#F16B79]/25"
                  : "bg-[#8B5CF6] text-white hover:bg-[#9B72F8] shadow-[0_0_18px_rgba(139,92,246,0.35)]"
            }`}
          >
            {busy ? <RefreshCw size={16} className="animate-spin" /> : active ? <RotateCcw size={16} /> : <Power size={16} />}
            {busy ?? (active ? "End Game Mode" : `Start ${mode.name}`)}
          </button>
        </div>

        {/* Live numbers */}
        <div className="grid grid-cols-4 gap-3">
          <Stat
            label="Free memory"
            value={`${formatMemory(scan.ramFreeMb)} of ${formatMemory(scan.ramTotalMb)}`}
            hint={picked.memoryMb > 0 ? `About ${formatMemory(picked.memoryMb)} can be freed` : "Nothing selected is running"}
          />
          <Stat label="Processes" value={`${scan.processCount}`} hint={`${picked.count} selected items running`} />
          <Stat
            label="Power plan"
            value={bitsumOk ? "Bitsum Highest Performance" : scan.powerPlan || "Unknown"}
            hint={bitsumOk ? "Kept on while Game Mode runs" : "Bitsum plan not found"}
            tone={bitsumOk ? "good" : "warn"}
          />
          <Stat
            label="Status"
            value={busy ? busy : active ? `${status?.mode || "On"} for ${formatElapsed(status?.startedAtMs ?? now, now)}` : "Off"}
            hint={active && status?.game ? `Game: ${status.game}` : active && status?.autoStarted ? "Started automatically" : "Ready"}
            tone={active ? "good" : undefined}
          />
        </div>

        {/* Admin helper */}
        {config.elevate === "skip" ? (
          <div className="flex items-start gap-3 px-3.5 py-3 rounded-lg border border-[#272D3D] bg-[#10131D]">
            <Lock size={15} className="mt-0.5 text-[#707B94]" />
            <div className="text-[12px] text-[#A6AFC4] leading-snug">
              <span className="font-semibold text-[#F4F5FA]">Services are never stopped.</span> Game Mode only closes apps and applies tweaks. Change this under
              "Admin rights for services" below.
            </div>
          </div>
        ) : (
          <div
            className={`flex items-start gap-3 px-3.5 py-3 rounded-lg border ${
              helper.armed && helper.covers ? "border-[#35C98B]/30 bg-[#35C98B]/5" : "border-[#F4B860]/30 bg-[#F4B860]/5"
            }`}
          >
            <Lock size={15} className={`mt-0.5 ${helper.armed && helper.covers ? "text-[#35C98B]" : "text-[#F4B860]"}`} />
            <div className="flex-1 min-w-0 text-[12px] text-[#A6AFC4] leading-snug">
              <div className="text-[12.5px] font-semibold text-[#F4F5FA]">
                {helper.armed && helper.covers ? "Admin helper is running" : helper.armed ? "Admin helper needs an update" : "Admin helper is off"}
              </div>
              {helper.armed && helper.covers ? (
                <div className="mt-0.5">Services stop and come back without a Windows prompt, even when a game starts Game Mode for you.</div>
              ) : (
                <div className="mt-0.5">
                  {helper.armed
                    ? "You added services since it started. Restart it once (one Windows prompt)."
                    : `Services stay running until you start it. Start it here while you are at the desktop${scan.isAdmin ? "" : ": Windows asks once, and never in the middle of a game"}.`}
                  {selectedHasServices ? "" : " This mode does not stop any services right now."}
                </div>
              )}
            </div>
            {!(helper.armed && helper.covers) && (
              <button
                onClick={() => void armHelper()}
                disabled={arming}
                aria-label={helper.armed ? "Restart the admin helper" : "Start the admin helper"}
                className="h-8 px-3 rounded-md text-[12px] font-semibold bg-[#8B5CF6] text-white hover:bg-[#9B72F8] disabled:opacity-60 disabled:cursor-wait cursor-pointer shrink-0 flex items-center gap-1.5"
              >
                {arming && <RefreshCw size={12} className="animate-spin" />}
                {arming ? "Waiting for approval" : helper.armed ? "Restart helper" : "Start helper"}
              </button>
            )}
          </div>
        )}

        {/* Modes */}
        <section aria-label="Modes" className="flex flex-col gap-2.5">
          <div className="flex items-center gap-2 flex-wrap">
            {config.modes.map((m) => (
              <button
                key={m.id}
                onClick={() => {
                  setEditingId(m.id);
                  setConfirmDelete(false);
                }}
                aria-pressed={m.id === mode.id}
                className={`h-8 px-3 rounded-full text-[12.5px] font-medium flex items-center gap-1.5 border cursor-pointer transition-colors ${
                  m.id === mode.id
                    ? "bg-[#30244F] border-[#8B5CF6]/70 text-[#F4F5FA]"
                    : "bg-[#10131D] border-[#272D3D] text-[#A6AFC4] hover:border-[#3A4157] hover:text-[#F4F5FA]"
                }`}
              >
                {m.name}
                {m.id === config.activeMode && <Star size={11} className="fill-current text-[#F4B860]" aria-label="Default mode" />}
              </button>
            ))}
            <button
              onClick={addMode}
              aria-label="New mode"
              className="h-8 px-3 rounded-full text-[12.5px] font-medium flex items-center gap-1.5 border border-dashed border-[#3A4157] text-[#B79CFF] hover:border-[#8B5CF6] hover:text-white cursor-pointer"
            >
              <Plus size={13} /> New mode
            </button>
          </div>
          <div className="flex items-center gap-2 flex-wrap bg-[#10131D] border border-[#272D3D] rounded-lg px-3 py-2">
            <label className="text-[11px] uppercase tracking-wider text-[#707B94]" htmlFor="game-mode-name">
              Mode name
            </label>
            <input
              id="game-mode-name"
              key={mode.id}
              defaultValue={mode.name}
              maxLength={40}
              onBlur={(event) => renameMode(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") event.currentTarget.blur();
              }}
              className="h-7 w-44 px-2 rounded-md bg-[#0E111A] border border-[#272D3D] text-[12.5px] text-[#F4F5FA] outline-none focus:border-[#8B5CF6]"
            />
            <button
              onClick={() => void save({ ...config, activeMode: mode.id })}
              disabled={config.activeMode === mode.id}
              aria-label="Use this mode for auto start and the deck key"
              className={`h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border cursor-pointer disabled:cursor-default ${
                config.activeMode === mode.id
                  ? "border-[#F4B860]/40 text-[#F4B860] bg-[#F4B860]/8"
                  : "border-[#272D3D] text-[#A6AFC4] hover:border-[#8B5CF6] hover:text-white"
              }`}
            >
              <Star size={12} className={config.activeMode === mode.id ? "fill-current" : ""} />
              {config.activeMode === mode.id ? "Used for auto start and the deck key" : "Use for auto start and the deck key"}
            </button>
            <span className="text-[11px] text-[#707B94]">
              Deck key parameter: <span className="text-[#A6AFC4] font-mono">{mode.id}</span>
            </span>
            <button
              onClick={deleteMode}
              disabled={config.modes.length < 2}
              aria-label={confirmDelete ? "Confirm delete mode" : "Delete mode"}
              className={`ml-auto h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed ${
                confirmDelete ? "border-[#F16B79]/60 text-[#F16B79] bg-[#F16B79]/10" : "border-[#272D3D] text-[#A6AFC4] hover:border-[#F16B79]/50 hover:text-[#F16B79]"
              }`}
            >
              <Trash2 size={12} /> {confirmDelete ? "Click again to delete" : "Delete"}
            </button>
          </div>
        </section>

        {/* Targets */}
        {GROUPS.map((group) => {
          const all = scan.targets.filter((t) => t.group === group.id);
          const items = all.filter((t) => showIdle || t.running || t.custom || mode.targets.includes(t.id));
          if (all.length === 0) return null;
          const ids = items.map((t) => t.id);
          const onCount = items.filter((t) => mode.targets.includes(t.id)).length;
          return (
            <section key={group.id} aria-label={group.title} className="flex flex-col gap-2">
              <div className="flex items-end justify-between">
                <div className="flex items-center gap-2">
                  <group.Icon size={14} />
                  <h2 className="text-[13px] font-semibold text-[#F4F5FA]">{group.title}</h2>
                  <span className="text-[11px] text-[#707B94]">
                    {onCount} of {items.length} selected
                  </span>
                </div>
                <div className="flex items-center gap-3 text-[11px]">
                  <button onClick={() => setGroup(ids, true)} className="text-[#B79CFF] hover:text-white cursor-pointer">
                    Select all
                  </button>
                  <button onClick={() => setGroup(ids, false)} className="text-[#A6AFC4] hover:text-white cursor-pointer">
                    Select none
                  </button>
                </div>
              </div>
              <p className="text-[11.5px] text-[#707B94] -mt-1">{group.blurb}</p>
              {items.length > 0 ? (
                <div className="grid grid-cols-2 gap-2">{items.map(rowFor)}</div>
              ) : (
                <p className="text-[11.5px] text-[#707B94] italic">Nothing from this group is running right now.</p>
              )}
            </section>
          );
        })}
        {(showIdle || idleCount > 0) && (
          <button
            onClick={() => setShowIdle((v) => !v)}
            aria-pressed={showIdle}
            className="self-start text-[11.5px] text-[#B79CFF] hover:text-white cursor-pointer -mt-2"
          >
            {showIdle ? "Hide presets that are not running" : `Show ${idleCount} presets that are not running`}
          </button>
        )}

        {/* Your own targets */}
        <section aria-label="Add your own" className="flex flex-col gap-2.5">
          <div className="flex items-center gap-2">
            <Plus size={14} />
            <h2 className="text-[13px] font-semibold text-[#F4F5FA]">Add your own</h2>
            <span className="text-[11px] text-[#707B94]">Pick programs or services from this PC. They are added to {mode.name}.</span>
            {!pickerOpen && (
              <button
                onClick={() => setPickerOpen(true)}
                aria-label="Add a program or service"
                className="ml-auto h-7 px-3 rounded-md text-[12px] font-semibold bg-[#30244F] text-[#F4F5FA] border border-[#8B5CF6]/50 hover:bg-[#3B2C63] cursor-pointer flex items-center gap-1.5"
              >
                <Plus size={12} /> Add
              </button>
            )}
          </div>
          {pickerOpen && (
            <div className="bg-[#10131D] border border-[#272D3D] rounded-lg p-3 flex flex-col gap-3">
              <div className="flex items-center gap-2">
                {(["programs", "services"] as const).map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setPickerTab(tab)}
                    aria-pressed={pickerTab === tab}
                    className={`h-7 px-3 rounded-md text-[12px] border cursor-pointer ${
                      pickerTab === tab ? "bg-[#30244F] border-[#8B5CF6]/60 text-[#F4F5FA]" : "bg-[#0E111A] border-[#272D3D] text-[#A6AFC4] hover:border-[#3A4157]"
                    }`}
                  >
                    {tab === "programs" ? `Running programs (${scan.processes.length})` : `Running services (${scan.services.length})`}
                  </button>
                ))}
                <div className="relative ml-auto">
                  <Search size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-[#707B94]" />
                  <input
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    placeholder="Search"
                    aria-label="Search programs and services"
                    className="h-7 w-48 pl-7 pr-2 rounded-md bg-[#0E111A] border border-[#272D3D] text-[12px] text-[#F4F5FA] outline-none focus:border-[#8B5CF6]"
                  />
                </div>
                <button
                  onClick={resetPicker}
                  aria-label="Close the picker"
                  className="w-7 h-7 rounded-md flex items-center justify-center text-[#707B94] hover:text-white hover:bg-[#1C2130] cursor-pointer"
                >
                  <X size={14} />
                </button>
              </div>
              <div className="max-h-52 overflow-y-auto grid grid-cols-2 gap-1.5 pr-1">
                {pickerTab === "programs" &&
                  programRows.map((p) => {
                    const on = pickedProcs.includes(p.name);
                    return (
                      <button
                        key={p.name}
                        onClick={() => setPickedProcs((list) => (on ? list.filter((n) => n !== p.name) : [...list, p.name]))}
                        disabled={Boolean(p.takenBy)}
                        aria-label={`${p.name}${p.takenBy ? `, already closed by ${p.takenBy}` : on ? ", picked" : ""}`}
                        className={`text-left flex items-center gap-2.5 px-2.5 py-1.5 rounded-md border cursor-pointer disabled:opacity-45 disabled:cursor-not-allowed ${
                          on ? "bg-[#151126] border-[#8B5CF6]/45" : "bg-[#0E111A] border-[#272D3D] hover:border-[#3A4157]"
                        }`}
                      >
                        <Checkbox checked={on} />
                        <span className="min-w-0 flex-1">
                          <span className="block text-[12px] text-[#F4F5FA] truncate">{p.name}</span>
                          <span className="block text-[10.5px] text-[#707B94] truncate">
                            {p.takenBy ? `Already closed by ${p.takenBy}` : `${formatMemory(p.memoryMb)}${p.count > 1 ? ` · ${p.count} processes` : ""}`}
                          </span>
                        </span>
                      </button>
                    );
                  })}
                {pickerTab === "services" &&
                  serviceRows.map((s) => {
                    const on = pickedServices.includes(s.name);
                    return (
                      <button
                        key={s.name}
                        onClick={() => setPickedServices((list) => (on ? list.filter((n) => n !== s.name) : [...list, s.name]))}
                        aria-label={`${s.display}${on ? ", picked" : ""}`}
                        className={`text-left flex items-center gap-2.5 px-2.5 py-1.5 rounded-md border cursor-pointer ${
                          on ? "bg-[#151126] border-[#8B5CF6]/45" : "bg-[#0E111A] border-[#272D3D] hover:border-[#3A4157]"
                        }`}
                      >
                        <Checkbox checked={on} />
                        <span className="min-w-0 flex-1">
                          <span className="block text-[12px] text-[#F4F5FA] truncate">{s.display}</span>
                          <span className="block text-[10.5px] text-[#707B94] truncate">{s.name}</span>
                        </span>
                      </button>
                    );
                  })}
                {((pickerTab === "programs" && programRows.length === 0) || (pickerTab === "services" && serviceRows.length === 0)) && (
                  <p className="col-span-2 text-[12px] text-[#707B94] py-3 text-center">Nothing matches.</p>
                )}
              </div>
              <p className="text-[11px] text-[#707B94] leading-snug">
                Windows, anti-cheat, graphics drivers, audio and security programs and services are hidden and can never be added.
              </p>
              <div className="flex items-end gap-3 flex-wrap border-t border-[#272D3D] pt-3">
                <label className="flex flex-col gap-1 text-[11px] uppercase tracking-wider text-[#707B94]">
                  Name
                  <input
                    value={customLabel}
                    onChange={(event) => setCustomLabel(event.target.value)}
                    placeholder={pickedProcs[0] ?? pickedServices[0] ?? "e.g. Work apps"}
                    maxLength={40}
                    aria-label="Name for this target"
                    className="h-8 w-48 px-2 rounded-md bg-[#0E111A] border border-[#272D3D] text-[12.5px] normal-case tracking-normal text-[#F4F5FA] outline-none focus:border-[#8B5CF6]"
                  />
                </label>
                {pickedProcs.length > 0 && (
                  <>
                    <div className="flex flex-col gap-1">
                      <span className="text-[11px] uppercase tracking-wider text-[#707B94]">Shown under</span>
                      <div className="flex gap-1">
                        {(["apps", "background"] as const).map((g) => (
                          <button
                            key={g}
                            onClick={() => setCustomGroup(g)}
                            aria-pressed={customGroup === g}
                            className={`h-8 px-2.5 rounded-md text-[12px] border cursor-pointer ${
                              customGroup === g ? "bg-[#30244F] border-[#8B5CF6]/60 text-[#F4F5FA]" : "bg-[#0E111A] border-[#272D3D] text-[#A6AFC4]"
                            }`}
                          >
                            {g === "apps" ? "Apps" : "Background"}
                          </button>
                        ))}
                      </div>
                    </div>
                    <button
                      onClick={() => setCustomGraceful((v) => !v)}
                      aria-pressed={customGraceful}
                      className="h-8 px-2.5 rounded-md text-[12px] border border-[#272D3D] bg-[#0E111A] text-[#A6AFC4] hover:border-[#3A4157] flex items-center gap-2 cursor-pointer"
                    >
                      <Checkbox checked={customGraceful} /> Ask it to close first
                    </button>
                    <button
                      onClick={() => setCustomReopen((v) => !v)}
                      aria-pressed={customReopen}
                      className="h-8 px-2.5 rounded-md text-[12px] border border-[#272D3D] bg-[#0E111A] text-[#A6AFC4] hover:border-[#3A4157] flex items-center gap-2 cursor-pointer"
                    >
                      <Checkbox checked={customReopen} /> Reopen afterwards
                    </button>
                  </>
                )}
                <button
                  onClick={() => void addCustom()}
                  disabled={pickedProcs.length + pickedServices.length === 0}
                  aria-label="Add the picked items to this mode"
                  className="ml-auto h-8 px-4 rounded-md text-[12.5px] font-semibold bg-[#8B5CF6] text-white hover:bg-[#9B72F8] disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                >
                  Add {pickedProcs.length + pickedServices.length > 0 ? `${pickedProcs.length + pickedServices.length} picked` : ""}
                </button>
              </div>
            </div>
          )}
          {config.customTargets.length === 0 && !pickerOpen && (
            <p className="text-[11.5px] text-[#707B94]">Nothing added yet. Your own targets show up in the lists above with a Yours tag.</p>
          )}
        </section>

        {/* Tweaks */}
        <section aria-label="Tweaks" className="flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <Zap size={14} />
            <h2 className="text-[13px] font-semibold text-[#F4F5FA]">Tweaks while you play</h2>
          </div>
          <div className="grid grid-cols-2 gap-2">
            {scan.tweaks.map((tweak) => (
              <Toggle key={tweak.id} on={mode.tweaks.includes(tweak.id)} onClick={() => toggleTweak(tweak.id)} title={tweak.label} hint={tweak.note} label={tweak.label} />
            ))}
            <div className="flex items-start gap-3 px-3.5 py-2.5 rounded-lg border border-[#35C98B]/30 bg-[#35C98B]/5 col-span-2">
              <span className="mt-0.5 text-[#35C98B]">
                <Lock size={14} />
              </span>
              <span className="min-w-0">
                <span className="block text-[12.5px] font-semibold text-[#F4F5FA]">Always run on Bitsum Highest Performance</span>
                <span className="block text-[11.5px] text-[#A6AFC4] mt-0.5 leading-snug">
                  Game Mode never switches to another power plan, and puts Bitsum back if something else was active. Currently active: {scan.powerPlan || "unknown"}.
                </span>
              </span>
            </div>
          </div>
        </section>

        {/* Behaviour */}
        <section aria-label="Behaviour" className="flex flex-col gap-3 pb-6">
          <div className="flex items-center gap-2">
            <ShieldCheck size={14} />
            <h2 className="text-[13px] font-semibold text-[#F4F5FA]">How it starts and ends</h2>
            <span className="text-[11px] text-[#707B94] ml-auto">{savedAt > 0 ? "Saved" : "Changes save automatically"}</span>
          </div>
          <Toggle
            on={config.autoDetect}
            onClick={() => void save({ ...config, autoDetect: !config.autoDetect })}
            label="Start automatically when a game goes full-screen"
            title="Start automatically when a game goes full-screen"
            hint="Looks for a full-screen game from Steam, Riot, Epic, FACEIT and similar launchers and runs the mode marked with a star. It ends again 15 seconds after the game closes. The deck key works either way."
          />
          {config.autoDetect && (
            <div className="flex items-center gap-2 flex-wrap pl-1">
              <span className="text-[12px] text-[#A6AFC4]">Wait after the game appears:</span>
              {SETTLE_OPTIONS.map((seconds) => (
                <button
                  key={seconds}
                  onClick={() => void save({ ...config, settleSeconds: seconds })}
                  aria-pressed={config.settleSeconds === seconds}
                  className={`h-7 px-2.5 rounded-md text-[12px] border cursor-pointer ${
                    config.settleSeconds === seconds ? "bg-[#30244F] border-[#8B5CF6]/60 text-[#F4F5FA]" : "bg-[#10131D] border-[#272D3D] text-[#A6AFC4] hover:border-[#3A4157]"
                  }`}
                >
                  {seconds}s
                </button>
              ))}
              <span className="text-[11px] text-[#707B94]">Lets the game finish loading before anything is closed, then closes one app at a time.</span>
            </div>
          )}
          {config.elevate === "ask" && (
            <Toggle
              on={config.armOnLaunch}
              onClick={() => void save({ ...config, armOnLaunch: !config.armOnLaunch })}
              label="Start the admin helper when DeckForge opens"
              title="Start the admin helper when DeckForge opens"
              hint="One Windows prompt each time DeckForge starts, so automatic Game Mode can stop services later without any prompt."
            />
          )}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <div className="text-[11px] uppercase tracking-wider text-[#707B94] mb-1.5">When Game Mode ends</div>
              <div className="flex flex-col gap-1.5">
                {RESTORE_OPTIONS.map((option) => (
                  <button
                    key={option.id}
                    onClick={() => void save({ ...config, restore: option.id })}
                    aria-pressed={config.restore === option.id}
                    className={`text-left px-3 py-2 rounded-lg border cursor-pointer transition-colors ${
                      config.restore === option.id ? "bg-[#30244F] border-[#8B5CF6]/60" : "bg-[#10131D] border-[#272D3D] hover:border-[#3A4157]"
                    }`}
                  >
                    <span className="block text-[12.5px] font-medium text-[#F4F5FA]">{option.title}</span>
                    <span className="block text-[11px] text-[#A6AFC4]">{option.hint}</span>
                  </button>
                ))}
              </div>
            </div>
            <div>
              <div className="text-[11px] uppercase tracking-wider text-[#707B94] mb-1.5">Admin rights for services</div>
              <div className="flex flex-col gap-1.5">
                {ELEVATE_OPTIONS.map((option) => (
                  <button
                    key={option.id}
                    onClick={() => void save({ ...config, elevate: option.id })}
                    aria-pressed={config.elevate === option.id}
                    className={`text-left px-3 py-2 rounded-lg border cursor-pointer transition-colors ${
                      config.elevate === option.id ? "bg-[#30244F] border-[#8B5CF6]/60" : "bg-[#10131D] border-[#272D3D] hover:border-[#3A4157]"
                    }`}
                  >
                    <span className="block text-[12.5px] font-medium text-[#F4F5FA]">{option.title}</span>
                    <span className="block text-[11px] text-[#A6AFC4]">{option.hint}</span>
                  </button>
                ))}
              </div>
              <p className="text-[11px] text-[#707B94] mt-2 leading-snug">
                {scan.isAdmin
                  ? "DeckForge already has admin rights, so no prompt appears."
                  : "The helper is a small hidden script that can only stop the services listed in your modes. It puts them back when Game Mode ends or DeckForge closes."}
              </p>
            </div>
          </div>
        </section>
      </div>

      {/* Report column */}
      <aside className="border-l border-[#272D3D] bg-[#10131D] overflow-y-auto p-4 flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h2 className="text-[13px] font-semibold text-[#F4F5FA]">{active ? `${status?.mode || "Game Mode"} is on` : "Last run"}</h2>
          <button
            onClick={() => void refresh()}
            aria-label="Rescan this PC"
            title="Rescan this PC"
            className="w-7 h-7 rounded-md bg-[#151925] border border-[#272D3D] hover:border-[#8B5CF6] flex items-center justify-center text-[#A6AFC4] cursor-pointer"
          >
            <RefreshCw size={13} className={scanning ? "animate-spin" : ""} />
          </button>
        </div>

        {status && (status.active || status.notes.length > 0 || status.closed.length > 0) ? (
          <>
            {active && (
              <div className="grid grid-cols-2 gap-2">
                <Stat
                  label="Free memory"
                  value={formatMemory(status.ramFreeAfterMb)}
                  hint={freedMb > 0 ? `+${formatMemory(freedMb)} (was ${formatMemory(status.ramFreeBeforeMb)})` : `was ${formatMemory(status.ramFreeBeforeMb)}`}
                  tone={freedMb > 0 ? "good" : undefined}
                />
                <Stat label="Processes" value={`${status.processesAfter}`} hint={`was ${status.processesBefore}`} />
              </div>
            )}
            {status.closed.length > 0 && (
              <div>
                <div className="text-[11px] uppercase tracking-wider text-[#707B94] mb-1.5">Closed</div>
                <ul className="flex flex-col gap-1">
                  {status.closed.map((app) => (
                    <li key={app.id} className="flex items-center justify-between text-[12px] bg-[#151925] border border-[#272D3D] rounded-md px-2.5 py-1.5">
                      <span className="text-[#F4F5FA]">{app.label}</span>
                      <span className="text-[#A6AFC4]">{formatMemory(app.memoryMb)}</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {status.stoppedServices.length > 0 && (
              <div>
                <div className="text-[11px] uppercase tracking-wider text-[#707B94] mb-1.5">Services stopped</div>
                <div className="flex flex-wrap gap-1">
                  {status.stoppedServices.map((name) => (
                    <span key={name} className="text-[11px] px-1.5 py-0.5 rounded bg-[#151925] border border-[#272D3D] text-[#A6AFC4]">
                      {name}
                    </span>
                  ))}
                </div>
              </div>
            )}
            {status.failedServices.length > 0 && (
              <div className="flex items-start gap-2 text-[11.5px] text-[#F4B860] bg-[#F4B860]/8 border border-[#F4B860]/30 rounded-md px-2.5 py-2">
                <TriangleAlert size={13} className="mt-0.5 shrink-0" />
                <span>Could not stop: {status.failedServices.join(", ")}</span>
              </div>
            )}
            {status.notes.length > 0 && (
              <div>
                <div className="text-[11px] uppercase tracking-wider text-[#707B94] mb-1.5">Notes</div>
                <ul className="flex flex-col gap-1">
                  {status.notes.map((note, index) => (
                    <li key={`${index}-${note}`} className="text-[11.5px] text-[#A6AFC4] leading-snug">
                      {note}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </>
        ) : (
          <p className="text-[12px] text-[#A6AFC4] leading-relaxed">
            Nothing has run yet. Start a mode to see what it closed, how much memory it freed and which services it stopped.
          </p>
        )}

        <div className="mt-auto pt-4 border-t border-[#272D3D]">
          <div className="flex items-center gap-2 mb-2 text-[#35C98B]">
            <ShieldCheck size={14} />
            <span className="text-[12px] font-semibold">Never touched</span>
          </div>
          <ul className="flex flex-col gap-1">
            {scan.protected.map((line) => (
              <li key={line} className="text-[11.5px] text-[#A6AFC4] leading-snug">
                {line}
              </li>
            ))}
          </ul>
        </div>
      </aside>
    </div>
  );
};