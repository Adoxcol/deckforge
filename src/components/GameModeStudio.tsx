import React from "react";
import {
  Check,
  Download,
  Gamepad2,
  Lock,
  Plus,
  Power,
  RefreshCw,
  RotateCcw,
  Search,
  ShieldCheck,
  Star,
  Trash2,
  TriangleAlert,
  Upload,
  X,
  Zap,
} from "lucide-react";
import { deckforgeApi } from "../bridge/tauriBridge";
import { errorText } from "../lib/format";
import {
  buildModeFile,
  parseModeFile,
  pickJsonFile,
  placeImportedMode,
  slugify,
  type PlacedMode,
} from "../lib/shareFiles";
import type {
  GameModeConfig,
  GameModeCustomTarget,
  GameModeMode,
  GameModeRunningApp,
  GameModeScan,
  GameModeServicePick,
  GameModeStatus,
} from "../types/deckforge";

type Notify = (text: string, kind?: "success" | "info" | "warning") => void;

type PickTab = "apps" | "background" | "services";

const PICK_TABS: { id: PickTab; title: string; blurb: string }[] = [
  { id: "apps", title: "Open apps", blurb: "Programs with a window open right now. They are asked to close first, so they can save." },
  { id: "background", title: "Background processes", blurb: "Programs running without a window, such as tray apps and sync clients." },
  { id: "services", title: "Services", blurb: "Windows services that are running. They stop through the admin helper and start again afterwards." },
];

const prettyName = (name: string) => name.charAt(0).toUpperCase() + name.slice(1);

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

const Radio: React.FC<{ checked: boolean }> = ({ checked }) => (
  <span
    aria-hidden="true"
    className={`w-4 h-4 rounded-full border flex items-center justify-center shrink-0 ${checked ? "border-[#8B5CF6]" : "border-[#3A4157]"}`}
  >
    <span className={`w-2 h-2 rounded-full ${checked ? "bg-[#8B5CF6]" : "bg-transparent"}`} />
  </span>
);

const PickRow: React.FC<{ on: boolean; title: string; sub: string; label: string; badge?: string; onClick: () => void }> = ({
  on,
  title,
  sub,
  label,
  badge,
  onClick,
}) => (
  <button
    onClick={onClick}
    aria-pressed={on}
    aria-label={label}
    className={`text-left flex items-center gap-2.5 px-2.5 py-1.5 rounded-md border cursor-pointer transition-colors ${
      on ? "bg-[#151126] border-[#8B5CF6]/45" : "bg-[#0E111A] border-[#272D3D] hover:border-[#3A4157]"
    }`}
  >
    <Checkbox checked={on} />
    <span className="min-w-0 flex-1">
      <span className="block text-[12px] text-[#F4F5FA] truncate">{title}</span>
      <span className="block text-[10.5px] text-[#707B94] truncate">{sub}</span>
    </span>
    {badge && <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#30244F] text-[#B79CFF] border border-[#8B5CF6]/40 shrink-0">{badge}</span>}
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
  const [confirmDelete, setConfirmDelete] = React.useState(false);
  const [arming, setArming] = React.useState(false);
  const [pendingImport, setPendingImport] = React.useState<PlacedMode | null>(null);

  // The list of what is running on this PC.
  const [pickTab, setPickTab] = React.useState<PickTab>("apps");
  const [query, setQuery] = React.useState("");

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

  /** Ids of custom targets that no mode refers to any more. They are dropped with the last mode that used them. */
  const withoutUnusedCustoms = (next: GameModeConfig): GameModeConfig => {
    const used = new Set(next.modes.flatMap((m) => m.targets));
    return { ...next, customTargets: next.customTargets.filter((c) => used.has(c.id)) };
  };

  const toggleTarget = (id: string) => {
    if (!config || !mode) return;
    const on = mode.targets.includes(id);
    const modes = config.modes.map((m) =>
      m.id === mode.id ? { ...m, targets: on ? m.targets.filter((t) => t !== id) : [...m.targets, id] } : m
    );
    void save(withoutUnusedCustoms({ ...config, modes }));
  };

  const clearTargets = () => {
    if (!config || !mode) return;
    void save(withoutUnusedCustoms({ ...config, modes: config.modes.map((m) => (m.id === mode.id ? { ...m, targets: [] } : m)) }));
  };

  const updateCustom = (id: string, patch: Partial<GameModeCustomTarget>) => {
    if (!config) return;
    void save({ ...config, customTargets: config.customTargets.map((c) => (c.id === id ? { ...c, ...patch } : c)) });
  };

  /** Makes a new target from something the user ticked in the running list, and adds it to this mode. */
  const addPicked = async (draft: Omit<GameModeCustomTarget, "id">) => {
    if (!config || !mode) return;
    const taken = new Set([...config.customTargets.map((c) => c.id), ...scan!.targets.map((t) => t.id)]);
    let id = `my-${slug(draft.label)}`;
    let n = 2;
    while (taken.has(id)) id = `my-${slug(draft.label)}-${n++}`;
    await save({
      ...config,
      customTargets: [...config.customTargets, { ...draft, id }],
      modes: config.modes.map((m) => (m.id === mode.id ? { ...m, targets: [...m.targets, id] } : m)),
    });
    await refresh();
  };

  const toggleProgram = (app: GameModeRunningApp) => {
    if (app.takenById) {
      toggleTarget(app.takenById);
      return;
    }
    void addPicked({
      label: prettyName(app.name),
      group: app.hasWindow ? "apps" : "background",
      processes: [app.name],
      services: [],
      // A program without a window has nothing to ask, so it is ended straight away.
      graceful: app.hasWindow,
      reopen: true,
    });
  };

  const toggleService = (service: GameModeServicePick) => {
    const owner = scan?.targets.find((t) => t.services.some((s) => s.name.toLowerCase() === service.name.toLowerCase()));
    if (owner) {
      toggleTarget(owner.id);
      return;
    }
    void addPicked({ label: service.display || service.name, group: "background", processes: [], services: [service.name], graceful: false, reopen: false });
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
    const created: GameModeMode = { id: `mode-${n}`, name: `Mode ${n}`, targets: [...mode.targets], tweaks: [...mode.tweaks], powerPlan: mode.powerPlan };
    setEditingId(created.id);
    setConfirmDelete(false);
    void save({ ...config, modes: [...config.modes, created] });
  };

  const exportMode = async () => {
    if (!config || !mode) return;
    try {
      const where = await deckforgeApi.saveExportFile(
        `deckforge-game-mode-${slugify(mode.name)}.json`,
        JSON.stringify(buildModeFile(mode, config.customTargets), null, 2)
      );
      notify(`Saved "${mode.name}" to ${where}`, "success");
    } catch (error) {
      notify(`Could not export the mode: ${errorText(error)}`, "warning");
    }
  };

  const startImport = async () => {
    if (!config || !scan) return;
    const picked = await pickJsonFile();
    if (!picked) return;
    try {
      const parsed = parseModeFile(picked.text);
      setPendingImport(
        placeImportedMode(
          parsed,
          config.modes,
          config.customTargets,
          scan.targets.map((t) => t.id),
          scan.tweaks.map((t) => t.id),
          scan.powerPlans.map((p) => p.guid)
        )
      );
    } catch (error) {
      notify(`Could not import ${picked.name}: ${errorText(error)}`, "warning");
    }
  };

  const confirmImport = async () => {
    if (!config || !pendingImport) return;
    const placed = pendingImport;
    setPendingImport(null);
    setEditingId(placed.mode.id);
    setConfirmDelete(false);
    await save({ ...config, customTargets: placed.customTargets, modes: [...config.modes, placed.mode] });
    await refresh();
    notify(`Imported "${placed.mode.name}". Check what it closes before you start it.`, "success");
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

  if (!scan || !config || !mode) {
    return (
      <div className="flex-1 flex items-center justify-center text-sm text-[#A6AFC4]">Analysing your PC...</div>
    );
  }

  const freedMb = status ? Math.max(0, status.ramFreeAfterMb - status.ramFreeBeforeMb) : 0;
  const q = query.trim().toLowerCase();
  const hit = (...parts: string[]) => !q || parts.some((part) => part.toLowerCase().includes(q));
  const everyApp = scan.processes.filter((p) => p.hasWindow);
  const everyBackground = scan.processes.filter((p) => !p.hasWindow);
  const appRows = everyApp.filter((p) => hit(p.name, p.takenBy ?? ""));
  const backgroundRows = everyBackground.filter((p) => hit(p.name, p.takenBy ?? ""));
  const serviceRows = scan.services.filter((s) => hit(s.name, s.display));
  const ticked = (target: string | null) => target !== null && mode.targets.includes(target);
  const serviceOwner = (name: string) => scan.targets.find((t) => t.services.some((s) => s.name.toLowerCase() === name.toLowerCase()));
  const tabCount: Record<PickTab, number> = { apps: everyApp.length, background: everyBackground.length, services: scan.services.length };
  const tabPicked: Record<PickTab, number> = {
    apps: everyApp.filter((p) => ticked(p.takenById)).length,
    background: everyBackground.filter((p) => ticked(p.takenById)).length,
    services: scan.services.filter((s) => ticked(serviceOwner(s.name)?.id ?? null)).length,
  };
  const selectedTargets = scan.targets.filter((t) => mode.targets.includes(t.id));
  const selectedHasServices = selectedTargets.some((t) => t.needsAdmin);
  const launchersToAdd = scan.targets.filter((t) => !t.custom && !mode.targets.includes(t.id));
  const modePlan = scan.powerPlans.find((p) => p.guid === mode.powerPlan);
  const planMissing = mode.powerPlan !== "" && !modePlan;
  const helper = scan.helper;
  const tab = PICK_TABS.find((t) => t.id === pickTab) ?? PICK_TABS[0];

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
            value={scan.powerPlan || "Unknown"}
            hint={modePlan ? `${mode.name} switches to ${modePlan.name}` : planMissing ? "Chosen plan is not installed" : "Game Mode leaves it alone"}
            tone={planMissing ? "warn" : undefined}
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
            <button
              onClick={() => void startImport()}
              aria-label="Import mode from a file"
              className="h-8 px-3 rounded-full text-[12.5px] font-medium flex items-center gap-1.5 border border-dashed border-[#3A4157] text-[#A6AFC4] hover:border-[#8B5CF6] hover:text-white cursor-pointer"
            >
              <Download size={13} /> Import
            </button>
          </div>
          {pendingImport && (
            <div role="dialog" aria-label="Import a mode" className="bg-[#151126] border border-[#8B5CF6]/45 rounded-lg px-4 py-3 flex flex-col gap-2">
              <div className="text-[13px] font-semibold text-[#F4F5FA]">Import "{pendingImport.mode.name}"?</div>
              <p className="text-[11.5px] text-[#A6AFC4] leading-snug">
                A mode file decides what Game Mode closes and stops on this PC. Read this list first. System programs and your dev tools stay protected no matter what a file says.
              </p>
              <ul className="flex flex-col gap-1 max-h-40 overflow-y-auto">
                {pendingImport.mode.targets.length === 0 && <li className="text-[11.5px] text-[#707B94]">It closes nothing.</li>}
                {pendingImport.mode.targets.map((id) => {
                  const custom = pendingImport.customTargets.find((c) => c.id === id);
                  const known = scan.targets.find((t) => t.id === id);
                  const label = custom?.label ?? known?.label ?? id;
                  const programs = custom?.processes ?? known?.processes ?? [];
                  const services = custom?.services ?? known?.services.map((s) => s.name) ?? [];
                  return (
                    <li key={id} className="text-[11.5px] text-[#A6AFC4]">
                      <span className="text-[#F4F5FA] font-medium">{label}</span>
                      {programs.length > 0 && <span> · closes {programs.join(", ")}</span>}
                      {services.length > 0 && <span> · stops service {services.join(", ")}</span>}
                    </li>
                  );
                })}
              </ul>
              {pendingImport.mode.powerPlan && (
                <div className="text-[11.5px] text-[#A6AFC4]">
                  Switches the power plan to {scan.powerPlans.find((p) => p.guid === pendingImport.mode.powerPlan)?.name ?? "an installed plan"} while it runs.
                </div>
              )}
              {pendingImport.powerPlanDropped && (
                <div className="text-[11.5px] text-[#F4B860]">The power plan in the file is not installed on this PC, so it was left out.</div>
              )}
              {pendingImport.dropped > 0 && (
                <div className="text-[11.5px] text-[#F4B860]">
                  {pendingImport.dropped} item{pendingImport.dropped === 1 ? "" : "s"} in the file this version does not know, so they were left out.
                </div>
              )}
              <div className="flex items-center gap-2">
                <button
                  onClick={() => void confirmImport()}
                  className="h-8 px-3 rounded-md bg-[#8B5CF6] hover:bg-[#9B72F8] text-white text-[12.5px] font-semibold cursor-pointer"
                >
                  Import mode
                </button>
                <button
                  onClick={() => setPendingImport(null)}
                  className="h-8 px-3 rounded-md border border-[#272D3D] text-[#A6AFC4] hover:text-white text-[12.5px] cursor-pointer"
                >
                  Cancel
                </button>
              </div>
            </div>
          )}
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
              onClick={() => void exportMode()}
              aria-label="Export this mode to a file"
              className="ml-auto h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border border-[#272D3D] text-[#A6AFC4] hover:border-[#8B5CF6] hover:text-white cursor-pointer"
            >
              <Upload size={12} /> Export
            </button>
            <button
              onClick={deleteMode}
              disabled={config.modes.length < 2}
              aria-label={confirmDelete ? "Confirm delete mode" : "Delete mode"}
              className={`h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed ${
                confirmDelete ? "border-[#F16B79]/60 text-[#F16B79] bg-[#F16B79]/10" : "border-[#272D3D] text-[#A6AFC4] hover:border-[#F16B79]/50 hover:text-[#F16B79]"
              }`}
            >
              <Trash2 size={12} /> {confirmDelete ? "Click again to delete" : "Delete"}
            </button>
          </div>
        </section>

        {/* What is running right now */}
        <section aria-label="Pick what to close" className="flex flex-col gap-2.5">
          <div className="flex items-center gap-2">
            <h2 className="text-[13px] font-semibold text-[#F4F5FA]">Pick what {mode.name} closes</h2>
            <span className="text-[11px] text-[#707B94]">This is what is running on your PC right now. Tick anything you want out of the way.</span>
            <button
              onClick={() => void refresh()}
              aria-label="Refresh the list"
              className="ml-auto h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border border-[#272D3D] text-[#A6AFC4] hover:border-[#8B5CF6] hover:text-white cursor-pointer"
            >
              <RefreshCw size={12} className={scanning ? "animate-spin" : ""} /> Refresh
            </button>
          </div>
          <div className="bg-[#10131D] border border-[#272D3D] rounded-lg p-3 flex flex-col gap-3">
            <div className="flex items-center gap-2 flex-wrap">
              {PICK_TABS.map((t) => (
                <button
                  key={t.id}
                  onClick={() => setPickTab(t.id)}
                  aria-pressed={pickTab === t.id}
                  className={`h-7 px-3 rounded-md text-[12px] border cursor-pointer flex items-center gap-1.5 ${
                    pickTab === t.id ? "bg-[#30244F] border-[#8B5CF6]/60 text-[#F4F5FA]" : "bg-[#0E111A] border-[#272D3D] text-[#A6AFC4] hover:border-[#3A4157]"
                  }`}
                >
                  {t.title} ({tabCount[t.id]})
                  {tabPicked[t.id] > 0 && <span className="text-[#B79CFF]">{tabPicked[t.id]} picked</span>}
                </button>
              ))}
              <div className="relative ml-auto">
                <Search size={12} className="absolute left-2 top-1/2 -translate-y-1/2 text-[#707B94]" />
                <input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Search"
                  aria-label="Search what is running"
                  className="h-7 w-48 pl-7 pr-2 rounded-md bg-[#0E111A] border border-[#272D3D] text-[12px] text-[#F4F5FA] outline-none focus:border-[#8B5CF6]"
                />
              </div>
            </div>
            <p className="text-[11.5px] text-[#707B94] -mt-1">{tab.blurb}</p>
            <div className="max-h-80 overflow-y-auto grid grid-cols-2 gap-1.5 pr-1">
              {pickTab !== "services" &&
                (pickTab === "apps" ? appRows : backgroundRows).map((p) => {
                  const on = ticked(p.takenById);
                  const sub = [formatMemory(p.memoryMb), p.count > 1 ? `${p.count} processes` : "", p.takenBy && p.takenBy.toLowerCase() !== p.name ? `closed as ${p.takenBy}` : ""]
                    .filter(Boolean)
                    .join(" · ");
                  return (
                    <PickRow
                      key={p.name}
                      on={on}
                      title={prettyName(p.name)}
                      sub={sub}
                      label={`${prettyName(p.name)}, ${on ? "closes in this mode" : "stays open"}`}
                      onClick={() => toggleProgram(p)}
                    />
                  );
                })}
              {pickTab === "services" &&
                serviceRows.map((s) => {
                  const owner = serviceOwner(s.name);
                  const on = ticked(owner?.id ?? null);
                  return (
                    <PickRow
                      key={s.name}
                      on={on}
                      title={s.display || s.name}
                      sub={s.name}
                      badge="helper"
                      label={`${s.display || s.name}, ${on ? "stops in this mode" : "keeps running"}`}
                      onClick={() => toggleService(s)}
                    />
                  );
                })}
              {(pickTab === "apps" ? appRows : pickTab === "background" ? backgroundRows : serviceRows).length === 0 && (
                <p className="col-span-2 text-[12px] text-[#707B94] py-3 text-center">{q ? "Nothing matches your search." : "Nothing to show."}</p>
              )}
            </div>
            <p className="text-[11px] text-[#707B94] leading-snug">
              Windows itself, anti-cheat, graphics drivers, audio and security software and DeckForge are never listed and can never be picked.
            </p>
            {launchersToAdd.length > 0 && (
              <div className="flex items-center gap-2 flex-wrap border-t border-[#272D3D] pt-3">
                <span className="text-[11.5px] text-[#707B94]">Not open right now? Add a common launcher:</span>
                {launchersToAdd.map((t) => (
                  <button
                    key={t.id}
                    onClick={() => toggleTarget(t.id)}
                    aria-label={`Add ${t.label} to this mode`}
                    title={t.note}
                    className="h-7 px-2.5 rounded-full text-[12px] border border-dashed border-[#3A4157] text-[#A6AFC4] hover:border-[#8B5CF6] hover:text-white cursor-pointer flex items-center gap-1.5"
                  >
                    <Plus size={11} /> {t.label}
                    {t.running && <span className="w-1.5 h-1.5 rounded-full bg-[#35C98B]" aria-label="running" />}
                  </button>
                ))}
              </div>
            )}
          </div>
        </section>

        {/* What this mode will do */}
        <section aria-label={`In ${mode.name}`} className="flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <ShieldCheck size={14} />
            <h2 className="text-[13px] font-semibold text-[#F4F5FA]">In {mode.name}</h2>
            <span className="text-[11px] text-[#707B94]">
              {selectedTargets.length === 0 ? "Nothing yet" : `${selectedTargets.length} to close or stop`}
            </span>
            {selectedTargets.length > 0 && (
              <button onClick={clearTargets} className="ml-auto text-[11px] text-[#A6AFC4] hover:text-white cursor-pointer">
                Clear all
              </button>
            )}
          </div>
          {selectedTargets.length === 0 ? (
            <p className="text-[11.5px] text-[#707B94] leading-snug">
              Nothing is picked, so starting this mode only applies the power plan and tweaks below. Pick apps or services above, or add a launcher.
            </p>
          ) : (
            <div className="grid grid-cols-2 gap-2">
              {selectedTargets.map((t) => {
                const custom = config.customTargets.find((c) => c.id === t.id);
                const detail = t.custom
                  ? [t.processes.length ? `Programs: ${t.processes.join(", ")}` : "", t.services.length ? `Services: ${t.services.map((s) => s.name).join(", ")}` : ""]
                      .filter(Boolean)
                      .join(" · ")
                  : t.note;
                return (
                  <div key={t.id} className="relative bg-[#151126] border border-[#8B5CF6]/45 rounded-lg px-3.5 py-2.5 pr-10">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-[12.5px] font-semibold text-[#F4F5FA]">{t.label}</span>
                      {t.running ? (
                        <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#35C98B]/12 text-[#35C98B] border border-[#35C98B]/30">
                          {t.processCount > 0
                            ? `Running · ${formatMemory(t.memoryMb)}${t.processCount > 1 ? ` · ${t.processCount} processes` : ""}`
                            : "Service running"}
                        </span>
                      ) : (
                        <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#1A1F2E] text-[#707B94] border border-[#272D3D]">Not running</span>
                      )}
                      {t.needsAdmin && (
                        <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#F4B860]/10 text-[#F4B860] border border-[#F4B860]/30 flex items-center gap-1">
                          <Lock size={9} /> helper
                        </span>
                      )}
                    </div>
                    <div className="text-[11.5px] text-[#A6AFC4] mt-0.5 leading-snug">{detail}</div>
                    {custom && custom.processes.length > 0 && (
                      <div className="flex items-center gap-3 mt-1.5 flex-wrap">
                        <button
                          onClick={() => updateCustom(custom.id, { graceful: !custom.graceful })}
                          aria-pressed={custom.graceful}
                          className="flex items-center gap-1.5 text-[11px] text-[#A6AFC4] hover:text-white cursor-pointer"
                        >
                          <Checkbox checked={custom.graceful} /> Ask it to close first
                        </button>
                        <button
                          onClick={() => updateCustom(custom.id, { reopen: !custom.reopen })}
                          aria-pressed={custom.reopen}
                          className="flex items-center gap-1.5 text-[11px] text-[#A6AFC4] hover:text-white cursor-pointer"
                        >
                          <Checkbox checked={custom.reopen} /> Reopen afterwards
                        </button>
                      </div>
                    )}
                    <button
                      onClick={() => toggleTarget(t.id)}
                      aria-label={`Take ${t.label} out of this mode`}
                      title="Take it out of this mode"
                      className="absolute top-2 right-2 w-6 h-6 rounded-md flex items-center justify-center text-[#707B94] hover:text-[#F16B79] hover:bg-[#F16B79]/10 cursor-pointer"
                    >
                      <X size={13} />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </section>

        {/* Power plan */}
        <section aria-label="Power plan" className="flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <Zap size={14} />
            <h2 className="text-[13px] font-semibold text-[#F4F5FA]">Power plan</h2>
            <span className="text-[11px] text-[#707B94]">The plans installed on this PC. Windows is using {scan.powerPlan || "an unknown plan"} now.</span>
          </div>
          <div className="grid grid-cols-2 gap-2">
            {[{ guid: "", name: "Leave it as it is", active: false }, ...scan.powerPlans].map((plan) => {
              const on = mode.powerPlan === plan.guid || (plan.guid === "" && planMissing);
              return (
                <button
                  key={plan.guid || "leave"}
                  onClick={() => updateMode({ powerPlan: plan.guid })}
                  aria-pressed={on}
                  aria-label={plan.guid === "" ? "Leave the power plan alone" : `Use the ${plan.name} power plan`}
                  className={`text-left flex items-center gap-3 px-3.5 py-2.5 rounded-lg border cursor-pointer transition-colors ${
                    on ? "bg-[#151126] border-[#8B5CF6]/45" : "bg-[#10131D] border-[#272D3D] hover:border-[#3A4157]"
                  }`}
                >
                  <Radio checked={on} />
                  <span className="min-w-0 flex-1">
                    <span className="block text-[12.5px] font-semibold text-[#F4F5FA] truncate">{plan.name}</span>
                    {plan.guid === "" && <span className="block text-[11px] text-[#707B94]">Game Mode never changes the plan</span>}
                  </span>
                  {plan.active && (
                    <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#35C98B]/12 text-[#35C98B] border border-[#35C98B]/30 shrink-0">In use now</span>
                  )}
                </button>
              );
            })}
          </div>
          <p className="text-[11px] text-[#707B94] leading-snug">
            {planMissing
              ? "The plan saved in this mode is not installed on this PC, so Game Mode leaves the plan alone. Pick one above."
              : modePlan
                ? `Game Mode switches to ${modePlan.name} when it starts and puts your earlier plan back when it ends.`
                : "Pick a plan if you want Game Mode to switch to it while you play."}
          </p>
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