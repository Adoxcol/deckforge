import React from "react";
import {
  Check,
  Download,
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
} from "lucide-react";
import { AppMark } from "./AppMark";
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
  GameModeStartApp,
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
      checked ? "bg-[var(--accent-primary)] border-[var(--accent-primary)] text-white" : "bg-[var(--bg-sidebar)] border-[var(--border-strong)] text-transparent"
    }`}
  >
    <Check size={11} strokeWidth={3} />
  </span>
);

const Stat: React.FC<{ label: string; value: string; hint?: string; tone?: "good" | "warn" }> = ({ label, value, hint, tone }) => (
  <div className="bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] rounded-xl px-3.5 py-3 min-w-0">
    <div className="text-[11px] font-medium text-[var(--text-muted)]">{label}</div>
    <div className={`mt-1 text-[15px] font-semibold tabular-nums truncate ${tone === "good" ? "text-[var(--status-success)]" : tone === "warn" ? "text-[var(--status-warning)]" : "text-[var(--text-primary)]"}`}>
      {value}
    </div>
    {hint && <div className="text-[11px] text-[var(--text-muted)] mt-0.5 truncate">{hint}</div>}
  </div>
);

const Toggle: React.FC<{ on: boolean; onClick: () => void; title: string; hint: string; label: string }> = ({ on, onClick, title, hint, label }) => (
  <button
    type="button"
    onClick={onClick}
    aria-pressed={on}
    aria-label={label}
    className="w-full text-left flex items-center gap-3 px-3 py-2.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-sidebar)] hover:border-[var(--border-strong)] cursor-pointer transition-colors"
  >
    <span className="min-w-0 flex-1">
      <span className="block text-[12.5px] font-semibold text-[var(--text-primary)]">{title}</span>
      <span className="block text-[11.5px] text-[var(--text-secondary)] mt-0.5 leading-snug">{hint}</span>
    </span>
    <span className={`w-9 h-5 rounded-full p-0.5 shrink-0 transition-colors ${on ? "bg-[var(--accent-primary)]" : "bg-[var(--border-strong)]"}`}>
      <span className={`block w-4 h-4 rounded-full bg-white shadow-xs transition-transform ${on ? "translate-x-4" : "translate-x-0"}`} />
    </span>
  </button>
);

const Step: React.FC<{ step: string; title: string; hint?: string; action?: React.ReactNode; children: React.ReactNode }> = ({ step, title, hint, action, children }) => (
  <section aria-label={title} className="rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-panel)]/35">
    <div className="flex items-start gap-3 px-4 py-3 border-b border-[var(--border-subtle)]">
      <span className="mt-0.5 w-5 h-5 rounded-full bg-[var(--accent-soft)] text-[var(--accent-text)] text-[11px] font-semibold flex items-center justify-center shrink-0">{step}</span>
      <div className="min-w-0 flex-1">
        <h2 className="text-[13px] font-semibold text-[var(--text-primary)]">{title}</h2>
        {hint && <p className="text-[11.5px] text-[var(--text-muted)] mt-0.5 leading-snug">{hint}</p>}
      </div>
      {action}
    </div>
    <div className="p-4 flex flex-col gap-3">{children}</div>
  </section>
);

const Radio: React.FC<{ checked: boolean }> = ({ checked }) => (
  <span
    aria-hidden="true"
    className={`w-4 h-4 rounded-full border flex items-center justify-center shrink-0 ${checked ? "border-[var(--accent-primary)]" : "border-[var(--border-strong)]"}`}
  >
    <span className={`w-2 h-2 rounded-full ${checked ? "bg-[var(--accent-primary)]" : "bg-transparent"}`} />
  </span>
);

const PickRow: React.FC<{ on: boolean; title: string; sub: string; label: string; badge?: string; mark?: string; path?: string; onClick: () => void }> = ({
  on,
  title,
  sub,
  label,
  badge,
  mark,
  path,
  onClick,
}) => (
  <button
    type="button"
    onClick={onClick}
    aria-pressed={on}
    aria-label={label}
    className={`text-left flex items-center gap-2.5 px-2.5 py-1.5 rounded-md border cursor-pointer transition-colors ${
      on ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/45" : "bg-[var(--bg-sidebar)] border-[var(--border-subtle)] hover:border-[var(--border-strong)]"
    }`}
  >
    <Checkbox checked={on} />
    {mark && <AppMark name={mark} path={path} />}
    <span className="min-w-0 flex-1">
      <span className="block text-[12px] text-[var(--text-primary)] truncate">{title}</span>
      <span className="block text-[10.5px] text-[var(--text-muted)] truncate">{sub}</span>
    </span>
    {badge && <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[var(--accent-soft)] text-[var(--accent-text)] border border-[var(--accent-primary)]/40 shrink-0">{badge}</span>}
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
    const created: GameModeMode = { id: `mode-${n}`, name: `Mode ${n}`, targets: [...mode.targets], tweaks: [...mode.tweaks], powerPlan: mode.powerPlan, startApps: (mode.startApps ?? []).map((a) => ({ ...a })) };
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
    let picked: { name: string; text: string } | null = null;
    try {
      picked = await pickJsonFile();
      if (!picked) return;
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
      notify(`Could not import ${picked?.name ?? "file"}: ${errorText(error)}`, "warning");
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
      <div className="flex-1 flex items-center justify-center text-sm text-[var(--text-secondary)]">Analysing your PC...</div>
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
    <div className="flex-1 min-h-0 overflow-y-auto lg:overflow-hidden lg:grid lg:grid-cols-[minmax(0,1fr)_280px] animate-screen-in">
      <div className="min-h-0 lg:overflow-y-auto px-6 py-5 flex flex-col gap-4">
        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <h1 className="text-lg font-semibold text-[var(--text-primary)]">Game Mode</h1>
              <span className={`h-5 px-2 rounded-full text-[11px] font-medium inline-flex items-center ${active ? "bg-[var(--status-success)]/15 text-[var(--status-success)]" : "bg-[var(--bg-elevated)] text-[var(--text-muted)]"}`}>
                {busy ? busy : active ? "On" : "Off"}
              </span>
            </div>
            <p className="text-[12.5px] text-[var(--text-secondary)] mt-1 leading-snug">
              {active
                ? `${status?.mode || mode.name} has been on for ${formatElapsed(status?.startedAtMs ?? now, now)}${status?.game ? ` · ${status.game}` : ""}.`
                : `${mode.name} ${selectedTargets.length === 0 ? "closes nothing" : `closes ${selectedTargets.length}`}${modePlan ? ` and switches to ${modePlan.name}` : ""}.`}
            </p>
          </div>
          <button
            type="button"
            onClick={() => void toggleGameMode()}
            disabled={Boolean(busy)}
            className={`h-10 px-4 rounded-lg text-sm font-semibold flex items-center gap-2 shrink-0 transition-colors cursor-pointer disabled:cursor-wait ${
              busy
                ? "bg-[var(--bg-elevated)] text-[var(--text-secondary)] border border-[var(--border-subtle)]"
                : active
                  ? "bg-[var(--status-error)]/15 text-[var(--status-error)] border border-[var(--status-error)]/50 hover:bg-[var(--status-error)]/25"
                  : "bg-[var(--accent-primary)] text-white hover:bg-[var(--accent-hover)]"
            }`}
          >
            {busy ? <RefreshCw size={16} className="animate-spin" /> : active ? <RotateCcw size={16} /> : <Power size={16} />}
            {busy ?? (active ? "End" : `Start ${mode.name}`)}
          </button>
        </div>

        <section aria-label="Modes" className="rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-panel)]/35 p-3 flex flex-col gap-3">
          <div className="flex items-center gap-2 flex-wrap">
            {config.modes.map((m) => (
              <button
                key={m.id}
                onClick={() => {
                  setEditingId(m.id);
                  setConfirmDelete(false);
                }}
                aria-pressed={m.id === mode.id}
                className={`h-8 px-3 rounded-lg text-[12.5px] font-medium flex items-center gap-1.5 border cursor-pointer transition-colors ${
                  m.id === mode.id
                    ? "bg-[var(--accent-primary)] border-transparent text-white"
                    : "bg-transparent border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--border-strong)] hover:text-[var(--text-primary)]"
                }`}
              >
                {m.name}
                {m.id === config.activeMode && <Star size={11} className="fill-current text-[var(--status-warning)]" aria-label="Default mode" />}
              </button>
            ))}
            <button
              onClick={addMode}
              aria-label="New mode"
              className="h-8 px-3 rounded-full text-[12.5px] font-medium flex items-center gap-1.5 border border-dashed border-[var(--border-strong)] text-[var(--accent-text)] hover:border-[var(--accent-primary)] hover:text-[var(--text-primary)] cursor-pointer"
            >
              <Plus size={13} /> New mode
            </button>
            <button
              onClick={() => void startImport()}
              aria-label="Import mode from a file"
              className="h-8 px-3 rounded-full text-[12.5px] font-medium flex items-center gap-1.5 border border-dashed border-[var(--border-strong)] text-[var(--text-secondary)] hover:border-[var(--accent-primary)] hover:text-[var(--text-primary)] cursor-pointer"
            >
              <Download size={13} /> Import
            </button>
          </div>
          {pendingImport && (
            <div role="dialog" aria-label="Import a mode" className="bg-[var(--accent-soft)] border border-[var(--accent-primary)]/45 rounded-lg px-4 py-3 flex flex-col gap-2">
              <div className="text-[13px] font-semibold text-[var(--text-primary)]">Import "{pendingImport.mode.name}"?</div>
              <p className="text-[11.5px] text-[var(--text-secondary)] leading-snug">
                A mode file decides what Game Mode closes and stops on this PC. Read this list first. System programs and your dev tools stay protected no matter what a file says.
              </p>
              <ul className="flex flex-col gap-1 max-h-40 overflow-y-auto">
                {pendingImport.mode.targets.length === 0 && <li className="text-[11.5px] text-[var(--text-muted)]">It closes nothing.</li>}
                {pendingImport.mode.targets.map((id) => {
                  const custom = pendingImport.customTargets.find((c) => c.id === id);
                  const known = scan.targets.find((t) => t.id === id);
                  const label = custom?.label ?? known?.label ?? id;
                  const programs = custom?.processes ?? known?.processes ?? [];
                  const services = custom?.services ?? known?.services.map((s) => s.name) ?? [];
                  return (
                    <li key={id} className="text-[11.5px] text-[var(--text-secondary)]">
                      <span className="text-[var(--text-primary)] font-medium">{label}</span>
                      {programs.length > 0 && <span> · closes {programs.join(", ")}</span>}
                      {services.length > 0 && <span> · stops service {services.join(", ")}</span>}
                    </li>
                  );
                })}
              </ul>
              {pendingImport.mode.powerPlan && (
                <div className="text-[11.5px] text-[var(--text-secondary)]">
                  Switches the power plan to {scan.powerPlans.find((p) => p.guid === pendingImport.mode.powerPlan)?.name ?? "an installed plan"} while it runs.
                </div>
              )}
              {pendingImport.powerPlanDropped && (
                <div className="text-[11.5px] text-[var(--status-warning)]">The power plan in the file is not installed on this PC, so it was left out.</div>
              )}
              {pendingImport.dropped > 0 && (
                <div className="text-[11.5px] text-[var(--status-warning)]">
                  {pendingImport.dropped} item{pendingImport.dropped === 1 ? "" : "s"} in the file this version does not know, so they were left out.
                </div>
              )}
              <div className="flex items-center gap-2">
                <button
                  onClick={() => void confirmImport()}
                  className="h-8 px-3 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white text-[12.5px] font-semibold cursor-pointer"
                >
                  Import mode
                </button>
                <button
                  onClick={() => setPendingImport(null)}
                  className="h-8 px-3 rounded-md border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] text-[12.5px] cursor-pointer"
                >
                  Cancel
                </button>
              </div>
            </div>
          )}
          <div className="flex items-center gap-2 flex-wrap pt-3 border-t border-[var(--border-subtle)]">
            <label className="sr-only" htmlFor="game-mode-name">
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
              className="h-8 w-48 px-2.5 rounded-md bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] text-[13px] font-medium text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)]"
            />
            <button
              onClick={() => void save({ ...config, activeMode: mode.id })}
              disabled={config.activeMode === mode.id}
              aria-label="Use this mode for auto start and the deck key"
              className={`h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border cursor-pointer disabled:cursor-default ${
                config.activeMode === mode.id
                  ? "border-[var(--status-warning)]/40 text-[var(--status-warning)] bg-[var(--status-warning)]/8"
                  : "border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--accent-primary)] hover:text-[var(--text-primary)]"
              }`}
            >
              <Star size={12} className={config.activeMode === mode.id ? "fill-current" : ""} />
              {config.activeMode === mode.id ? "Used for auto start and the deck key" : "Use for auto start and the deck key"}
            </button>
            <span className="text-[11px] text-[var(--text-muted)]">
              Deck key parameter: <span className="text-[var(--text-secondary)] font-mono">{mode.id}</span>
            </span>
            <button
              onClick={() => void exportMode()}
              aria-label="Export this mode to a file"
              className="ml-auto h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--accent-primary)] hover:text-[var(--text-primary)] cursor-pointer"
            >
              <Upload size={12} /> Export
            </button>
            <button
              onClick={deleteMode}
              disabled={config.modes.length < 2}
              aria-label={confirmDelete ? "Confirm delete mode" : "Delete mode"}
              className={`h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed ${
                confirmDelete ? "border-[var(--status-error)]/60 text-[var(--status-error)] bg-[var(--status-error)]/10" : "border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--status-error)]/50 hover:text-[var(--status-error)]"
              }`}
            >
              <Trash2 size={12} /> {confirmDelete ? "Click again to delete" : "Delete"}
            </button>
          </div>
        </section>

        {/* What is running right now */}
        <Step
          step="1"
          title={`Close in ${mode.name}`}
          hint="Tick what should get out of the way. Windows, anti-cheat, drivers and DeckForge can never be picked."
          action={
            <button
              type="button"
              onClick={() => void refresh()}
              aria-label="Refresh the list"
              className="h-7 px-2.5 rounded-md text-[12px] flex items-center gap-1.5 border border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--accent-primary)] hover:text-[var(--text-primary)] cursor-pointer"
            >
              <RefreshCw size={12} className={scanning ? "animate-spin" : ""} /> Refresh
            </button>
          }
        >
            <div className="flex items-center gap-2 flex-wrap">
              {PICK_TABS.map((t) => (
                <button
                  key={t.id}
                  onClick={() => setPickTab(t.id)}
                  aria-pressed={pickTab === t.id}
                  className={`h-7 px-3 rounded-md text-[12px] border cursor-pointer flex items-center gap-1.5 ${
                    pickTab === t.id ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/60 text-[var(--text-primary)]" : "bg-[var(--bg-sidebar)] border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--border-strong)]"
                  }`}
                >
                  {t.title} ({tabCount[t.id]})
                  {tabPicked[t.id] > 0 && <span className="text-[var(--accent-text)]">{tabPicked[t.id]} picked</span>}
                </button>
              ))}
              <div className="relative ml-auto">
                <Search size={12} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)] pointer-events-none" />
                <input
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Search"
                  aria-label="Search what is running"
                  className="h-7 w-48 pl-7 pr-6 rounded-md bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] text-[12px] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] transition-colors"
                />
                {query && (
                  <button
                    type="button"
                    aria-label="Clear running apps search"
                    onClick={() => setQuery("")}
                    className="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
                  >
                    <X size={11} />
                  </button>
                )}
              </div>
            </div>
            <p className="text-[11.5px] text-[var(--text-muted)] -mt-1">{tab.blurb}</p>
            <div className="max-h-80 overflow-y-auto grid grid-cols-1 lg:grid-cols-2 gap-1.5 pr-1">
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
                      mark={p.name}
                      path={p.path}
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
                <p className="col-span-2 text-[12px] text-[var(--text-muted)] py-3 text-center">{q ? "Nothing matches your search." : "Nothing to show."}</p>
              )}
            </div>
            <p className="text-[11px] text-[var(--text-muted)] leading-snug">
              Windows itself, anti-cheat, graphics drivers, audio and security software and DeckForge are never listed and can never be picked.
            </p>
            {launchersToAdd.length > 0 && (
              <div className="flex items-center gap-2 flex-wrap border-t border-[var(--border-subtle)] pt-3">
                <span className="text-[11.5px] text-[var(--text-muted)]">Not open right now? Add a common launcher:</span>
                {launchersToAdd.map((t) => (
                  <button
                    key={t.id}
                    onClick={() => toggleTarget(t.id)}
                    aria-label={`Add ${t.label} to this mode`}
                    title={t.note}
                    className="h-7 px-2.5 rounded-full text-[12px] border border-dashed border-[var(--border-strong)] text-[var(--text-secondary)] hover:border-[var(--accent-primary)] hover:text-[var(--text-primary)] cursor-pointer flex items-center gap-1.5"
                  >
                    <Plus size={11} />
                    <AppMark name={t.processes[0] || t.label} size={14} />
                    {t.label}
                    {t.running && <span className="w-1.5 h-1.5 rounded-full bg-[var(--status-success)]" aria-label="running" />}
                  </button>
                ))}
              </div>
            )}
          <div className="border-t border-[var(--border-subtle)] pt-3 flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <h3 className="text-[12.5px] font-semibold text-[var(--text-primary)]">Chosen</h3>
            <span className="text-[11px] text-[var(--text-muted)]">
              {selectedTargets.length === 0 ? "Nothing yet" : `${selectedTargets.length} to close or stop`}
            </span>
            {selectedTargets.length > 0 && (
              <button onClick={clearTargets} className="ml-auto text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer">
                Clear all
              </button>
            )}
          </div>
          {selectedTargets.length === 0 ? (
            <p className="text-[11.5px] text-[var(--text-muted)] leading-snug">
              Nothing is picked, so starting this mode only applies the power plan and tweaks below. Pick apps or services above, or add a launcher.
            </p>
          ) : (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-2">
              {selectedTargets.map((t) => {
                const custom = config.customTargets.find((c) => c.id === t.id);
                const detail = t.custom
                  ? [t.processes.length ? `Programs: ${t.processes.join(", ")}` : "", t.services.length ? `Services: ${t.services.map((s) => s.name).join(", ")}` : ""]
                      .filter(Boolean)
                      .join(" · ")
                  : t.note;
                return (
                  <div key={t.id} className="relative bg-[var(--bg-sidebar)] border border-[var(--border-subtle)] rounded-lg px-3 py-2 pr-9">
                    <div className="flex items-center gap-2 flex-wrap">
                      <AppMark name={t.processes[0] || t.label} size={18} />
                      <span className="text-[12.5px] font-semibold text-[var(--text-primary)]">{t.label}</span>
                      {t.running ? (
                        <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[var(--status-success)]/12 text-[var(--status-success)] border border-[var(--status-success)]/30">
                          {t.processCount > 0
                            ? `Running · ${formatMemory(t.memoryMb)}${t.processCount > 1 ? ` · ${t.processCount} processes` : ""}`
                            : "Service running"}
                        </span>
                      ) : (
                        <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[#1A1F2E] text-[var(--text-muted)] border border-[var(--border-subtle)]">Not running</span>
                      )}
                      {t.needsAdmin && (
                        <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[var(--status-warning)]/10 text-[var(--status-warning)] border border-[var(--status-warning)]/30 flex items-center gap-1">
                          <Lock size={9} /> helper
                        </span>
                      )}
                    </div>
                    <div className="text-[11.5px] text-[var(--text-secondary)] mt-0.5 leading-snug">{detail}</div>
                    {custom && custom.processes.length > 0 && (
                      <div className="flex items-center gap-3 mt-1.5 flex-wrap">
                        <button
                          onClick={() => updateCustom(custom.id, { graceful: !custom.graceful })}
                          aria-pressed={custom.graceful}
                          className="flex items-center gap-1.5 text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer"
                        >
                          <Checkbox checked={custom.graceful} /> Ask it to close first
                        </button>
                        <button
                          onClick={() => updateCustom(custom.id, { reopen: !custom.reopen })}
                          aria-pressed={custom.reopen}
                          className="flex items-center gap-1.5 text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer"
                        >
                          <Checkbox checked={custom.reopen} /> Reopen afterwards
                        </button>
                      </div>
                    )}
                    <button
                      onClick={() => toggleTarget(t.id)}
                      aria-label={`Take ${t.label} out of this mode`}
                      title="Take it out of this mode"
                      className="absolute top-2 right-2 w-6 h-6 rounded-md flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--status-error)] hover:bg-[var(--status-error)]/10 cursor-pointer"
                    >
                      <X size={13} />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
          </div>
        </Step>

        <Step step="2" title="Open these" hint="Apps you want running while you play, like Outplayed or ExitLag.">
          <StartAppsSection
            apps={mode.startApps ?? []}
            running={scan.processes}
            onChange={(startApps) => updateMode({ startApps })}
          />
        </Step>

        <Step step="3" title="While you play" hint={`Windows is using ${scan.powerPlan || "an unknown plan"} now.`}>
          <div className="text-[12px] font-medium text-[var(--text-secondary)]">Power plan</div>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-2">
            {[{ guid: "", name: "Leave it as it is", active: false }, ...scan.powerPlans].map((plan) => {
              const on = mode.powerPlan === plan.guid || (plan.guid === "" && planMissing);
              return (
                <button
                  key={plan.guid || "leave"}
                  onClick={() => updateMode({ powerPlan: plan.guid })}
                  aria-pressed={on}
                  aria-label={plan.guid === "" ? "Leave the power plan alone" : `Use the ${plan.name} power plan`}
                  className={`text-left flex items-center gap-3 px-3.5 py-2.5 rounded-lg border cursor-pointer transition-colors ${
                    on ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/45" : "bg-[var(--bg-sidebar)] border-[var(--border-subtle)] hover:border-[var(--border-strong)]"
                  }`}
                >
                  <Radio checked={on} />
                  <span className="min-w-0 flex-1">
                    <span className="block text-[12.5px] font-semibold text-[var(--text-primary)] truncate">{plan.name}</span>
                    {plan.guid === "" && <span className="block text-[11px] text-[var(--text-muted)]">Game Mode never changes the plan</span>}
                  </span>
                  {plan.active && (
                    <span className="text-[10.5px] px-1.5 py-0.5 rounded bg-[var(--status-success)]/12 text-[var(--status-success)] border border-[var(--status-success)]/30 shrink-0">In use now</span>
                  )}
                </button>
              );
            })}
          </div>
          <p className="text-[11px] text-[var(--text-muted)] leading-snug">
            {planMissing
              ? "The plan saved in this mode is not installed on this PC, so Game Mode leaves the plan alone. Pick one above."
              : modePlan
                ? `Game Mode switches to ${modePlan.name} when it starts and puts your earlier plan back when it ends.`
                : "Pick a plan if you want Game Mode to switch to it while you play."}
          </p>
          <div className="border-t border-[var(--border-subtle)] pt-3 flex flex-col gap-2">
            <div className="text-[12px] font-medium text-[var(--text-secondary)]">Tweaks</div>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-2">
              {scan.tweaks.map((tweak) => (
                <Toggle key={tweak.id} on={mode.tweaks.includes(tweak.id)} onClick={() => toggleTweak(tweak.id)} title={tweak.label} hint={tweak.note} label={tweak.label} />
              ))}
            </div>
          </div>
        </Step>

        <Step step="4" title="When it starts and ends" hint={savedAt > 0 ? "Saved" : "Changes save automatically"}>
          <Toggle
            on={config.autoDetect}
            onClick={() => void save({ ...config, autoDetect: !config.autoDetect })}
            label="Start automatically when a game goes full-screen"
            title="Start automatically when a game goes full-screen"
            hint="Looks for a full-screen game from Steam, Riot, Epic, FACEIT and similar launchers and runs the mode marked with a star. It ends again 15 seconds after the game closes. The deck key works either way."
          />
          {config.autoDetect && (
            <div className="flex items-center gap-2 flex-wrap pl-1">
              <span className="text-[12px] text-[var(--text-secondary)]">Wait after the game appears:</span>
              {SETTLE_OPTIONS.map((seconds) => (
                <button
                  key={seconds}
                  onClick={() => void save({ ...config, settleSeconds: seconds })}
                  aria-pressed={config.settleSeconds === seconds}
                  className={`h-7 px-2.5 rounded-md text-[12px] border cursor-pointer ${
                    config.settleSeconds === seconds ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/60 text-[var(--text-primary)]" : "bg-[var(--bg-sidebar)] border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--border-strong)]"
                  }`}
                >
                  {seconds}s
                </button>
              ))}
              <span className="text-[11px] text-[var(--text-muted)]">Lets the game finish loading before anything is closed, then closes one app at a time.</span>
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
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <div className="text-[11px] uppercase tracking-wider text-[var(--text-muted)] mb-1.5">When Game Mode ends</div>
              <div className="flex flex-col gap-1.5">
                {RESTORE_OPTIONS.map((option) => (
                  <button
                    key={option.id}
                    onClick={() => void save({ ...config, restore: option.id })}
                    aria-pressed={config.restore === option.id}
                    className={`text-left px-3 py-2 rounded-lg border cursor-pointer transition-colors ${
                      config.restore === option.id ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/60" : "bg-[var(--bg-sidebar)] border-[var(--border-subtle)] hover:border-[var(--border-strong)]"
                    }`}
                  >
                    <span className="block text-[12.5px] font-medium text-[var(--text-primary)]">{option.title}</span>
                    <span className="block text-[11px] text-[var(--text-secondary)]">{option.hint}</span>
                  </button>
                ))}
              </div>
            </div>
            <div>
              <div className="text-[11px] uppercase tracking-wider text-[var(--text-muted)] mb-1.5">Admin rights for services</div>
              <div className="flex flex-col gap-1.5">
                {ELEVATE_OPTIONS.map((option) => (
                  <button
                    key={option.id}
                    onClick={() => void save({ ...config, elevate: option.id })}
                    aria-pressed={config.elevate === option.id}
                    className={`text-left px-3 py-2 rounded-lg border cursor-pointer transition-colors ${
                      config.elevate === option.id ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/60" : "bg-[var(--bg-sidebar)] border-[var(--border-subtle)] hover:border-[var(--border-strong)]"
                    }`}
                  >
                    <span className="block text-[12.5px] font-medium text-[var(--text-primary)]">{option.title}</span>
                    <span className="block text-[11px] text-[var(--text-secondary)]">{option.hint}</span>
                  </button>
                ))}
              </div>
              <p className="text-[11px] text-[var(--text-muted)] mt-2 leading-snug">
                {scan.isAdmin
                  ? "DeckForge already has admin rights, so no prompt appears."
                  : "The helper is a small hidden script that can only stop the services listed in your modes. It puts them back when Game Mode ends or DeckForge closes."}
              </p>
            </div>
          </div>
        </Step>
      </div>

      {/* Report column */}
      <aside className="min-h-0 border-t lg:border-t-0 lg:border-l border-[var(--border-subtle)] bg-[var(--bg-sidebar)] lg:overflow-y-auto p-4 flex flex-col gap-4">
        <div className="rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-panel)]/50 p-3 flex flex-col gap-2.5">
          <div className="text-[11px] font-medium tracking-wide text-[var(--text-muted)]">This PC</div>
          <div className="flex items-baseline justify-between gap-3">
            <span className="text-[12px] text-[var(--text-secondary)]">Free memory</span>
            <span className="text-[12.5px] font-semibold tabular-nums text-[var(--text-primary)]">{formatMemory(scan.ramFreeMb)}</span>
          </div>
          <div className="flex items-baseline justify-between gap-3">
            <span className="text-[12px] text-[var(--text-secondary)]">Can be freed</span>
            <span className="text-[12.5px] font-semibold tabular-nums text-[var(--text-primary)]">{picked.memoryMb > 0 ? formatMemory(picked.memoryMb) : "—"}</span>
          </div>
          <div className="flex items-baseline justify-between gap-3">
            <span className="text-[12px] text-[var(--text-secondary)]">Processes</span>
            <span className="text-[12.5px] font-semibold tabular-nums text-[var(--text-primary)]">{scan.processCount}</span>
          </div>
          <div className="flex items-baseline justify-between gap-3">
            <span className="text-[12px] text-[var(--text-secondary)]">Power plan</span>
            <span className={`text-[12.5px] font-semibold text-right truncate ${planMissing ? "text-[var(--status-warning)]" : "text-[var(--text-primary)]"}`}>{scan.powerPlan || "Unknown"}</span>
          </div>
        </div>

        <div className={`rounded-xl border p-3 flex flex-col gap-2 ${config.elevate === "skip" ? "border-[var(--border-subtle)]" : helper.armed && helper.covers ? "border-[var(--status-success)]/30" : "border-[var(--status-warning)]/35"}`}>
          <div className="flex items-center gap-2">
            <Lock size={14} className={config.elevate === "skip" || (helper.armed && helper.covers) ? "text-[var(--text-muted)]" : "text-[var(--status-warning)]"} />
            <h2 className="text-[12.5px] font-semibold text-[var(--text-primary)]">
              {config.elevate === "skip" ? "Services stay running" : helper.armed && helper.covers ? "Admin helper is running" : helper.armed ? "Helper needs an update" : "Admin helper is off"}
            </h2>
          </div>
          <p className="text-[11.5px] text-[var(--text-secondary)] leading-snug">
            {config.elevate === "skip"
              ? "This setup only closes apps and applies tweaks. Turn the helper on in step 4 to stop services."
              : helper.armed && helper.covers
                ? "Services stop and come back without another Windows prompt."
                : helper.armed
                  ? "You added services since it started. Restart it once."
                  : `Start it at the desktop${scan.isAdmin ? "" : " — Windows asks once"}.${selectedHasServices ? "" : " This mode does not stop any services right now."}`}
          </p>
          {config.elevate !== "skip" && !(helper.armed && helper.covers) && (
            <button
              type="button"
              onClick={() => void armHelper()}
              disabled={arming}
              aria-label={helper.armed ? "Restart the admin helper" : "Start the admin helper"}
              className="h-8 px-3 rounded-md text-[12px] font-semibold bg-[var(--accent-primary)] text-white hover:bg-[var(--accent-hover)] disabled:opacity-60 disabled:cursor-wait cursor-pointer self-start flex items-center gap-1.5"
            >
              {arming && <RefreshCw size={12} className="animate-spin" />}
              {arming ? "Waiting for approval" : helper.armed ? "Restart helper" : "Start helper"}
            </button>
          )}
        </div>

        <div className="flex items-center justify-between">
          <h2 className="text-[13px] font-semibold text-[var(--text-primary)]">{active ? `${status?.mode || "Game Mode"} is on` : "Last run"}</h2>
          <button
            type="button"
            onClick={() => void refresh()}
            aria-label="Rescan this PC"
            title="Rescan this PC"
            className="w-7 h-7 rounded-md bg-[var(--bg-panel)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] flex items-center justify-center text-[var(--text-secondary)] cursor-pointer"
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
                <div className="text-[11px] uppercase tracking-wider text-[var(--text-muted)] mb-1.5">Closed</div>
                <ul className="flex flex-col gap-1">
                  {status.closed.map((app) => (
                    <li key={app.id} className="flex items-center justify-between text-[12px] bg-[var(--bg-panel)] border border-[var(--border-subtle)] rounded-md px-2.5 py-1.5">
                      <span className="text-[var(--text-primary)]">{app.label}</span>
                      <span className="text-[var(--text-secondary)]">{formatMemory(app.memoryMb)}</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {status.stoppedServices.length > 0 && (
              <div>
                <div className="text-[11px] uppercase tracking-wider text-[var(--text-muted)] mb-1.5">Services stopped</div>
                <div className="flex flex-wrap gap-1">
                  {status.stoppedServices.map((name) => (
                    <span key={name} className="text-[11px] px-1.5 py-0.5 rounded bg-[var(--bg-panel)] border border-[var(--border-subtle)] text-[var(--text-secondary)]">
                      {name}
                    </span>
                  ))}
                </div>
              </div>
            )}
            {status.failedServices.length > 0 && (
              <div className="flex items-start gap-2 text-[11.5px] text-[var(--status-warning)] bg-[var(--status-warning)]/8 border border-[var(--status-warning)]/30 rounded-md px-2.5 py-2">
                <TriangleAlert size={13} className="mt-0.5 shrink-0" />
                <span>Could not stop: {status.failedServices.join(", ")}</span>
              </div>
            )}
            {status.notes.length > 0 && (
              <div>
                <div className="text-[11px] uppercase tracking-wider text-[var(--text-muted)] mb-1.5">Notes</div>
                <ul className="flex flex-col gap-1">
                  {status.notes.map((note, index) => (
                    <li key={`${index}-${note}`} className="text-[11.5px] text-[var(--text-secondary)] leading-snug">
                      {note}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </>
        ) : (
          <p className="text-[12px] text-[var(--text-secondary)] leading-relaxed">
            Nothing has run yet. Start a mode to see what it closed, how much memory it freed and which services it stopped.
          </p>
        )}

        <div className="mt-auto pt-4 border-t border-[var(--border-subtle)]">
          <div className="flex items-center gap-2 mb-2 text-[var(--status-success)]">
            <ShieldCheck size={14} />
            <span className="text-[12px] font-semibold">Never touched</span>
          </div>
          <ul className="flex flex-col gap-1">
            {scan.protected.map((line) => (
              <li key={line} className="text-[11.5px] text-[var(--text-secondary)] leading-snug">
                {line}
              </li>
            ))}
          </ul>
        </div>
      </aside>
    </div>
  );
};

const exeName = (path: string) => path.split(/[\\/]/).pop() ?? path;
const stemOf = (path: string) => exeName(path).replace(/\.exe$/i, "");

/** Programs Game Mode starts when it turns on, such as Outplayed or ExitLag. */
const StartAppsSection: React.FC<{
  apps: GameModeStartApp[];
  running: GameModeRunningApp[];
  onChange: (apps: GameModeStartApp[]) => void;
}> = ({ apps, running, onChange }) => {
  const [typed, setTyped] = React.useState("");
  const [picking, setPicking] = React.useState(false);
  const has = (path: string) => apps.some((a) => a.path.toLowerCase() === path.toLowerCase());
  const add = (path: string) => {
    const clean = path.trim().replace(/^"|"$/g, "");
    if (!/\.exe$/i.test(clean) || has(clean) || apps.length >= 10) return false;
    onChange([...apps, { label: stemOf(clean), path: clean, args: "", closeOnEnd: false }]);
    return true;
  };
  const patch = (i: number, change: Partial<GameModeStartApp>) => onChange(apps.map((a, j) => (j === i ? { ...a, ...change } : a)));
  const choices = running.filter((p) => p.path && /\.exe$/i.test(p.path) && !has(p.path));
  const typedOk = /\.exe"?$/i.test(typed.trim());

  return (
    <div className="flex flex-col gap-3">
      {apps.length === 0 && (
        <div className="px-3.5 py-3 rounded-lg border border-dashed border-[var(--border-subtle)] text-[11.5px] text-[var(--text-muted)]">
          Nothing starts yet. Add an app that is running now, or paste the path to its .exe.
        </div>
      )}
      {apps.map((app, i) => (
        <div key={app.path} className="flex flex-col gap-2 px-3.5 py-2.5 rounded-lg border bg-[var(--bg-sidebar)] border-[var(--border-subtle)]">
          <div className="flex items-center gap-2">
            <AppMark name={app.label} path={app.path} size={22} />
            <input
              value={app.label}
              onChange={(e) => patch(i, { label: e.target.value.slice(0, 40) })}
              aria-label="Name"
              className="w-40 h-7 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-[12.5px] font-semibold text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)]"
            />
            <span className="flex-1 min-w-0 truncate text-[11px] text-[var(--text-muted)]" title={app.path}>{app.path}</span>
            <button
              onClick={() => onChange(apps.filter((_, j) => j !== i))}
              aria-label={`Remove ${app.label}`}
              className="p-1 rounded text-[var(--text-secondary)] hover:text-[var(--status-error)] cursor-pointer"
            >
              <Trash2 size={13} />
            </button>
          </div>
          <div className="flex items-center flex-wrap gap-3">
            <input
              value={app.args}
              onChange={(e) => patch(i, { args: e.target.value.slice(0, 300) })}
              placeholder="Arguments (optional), e.g. -launchapp <id>"
              aria-label="Arguments"
              className="flex-1 h-7 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-[11.5px] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] placeholder:text-[var(--text-muted)]"
            />
            <label className="flex items-center gap-1.5 text-[11.5px] text-[var(--text-secondary)] cursor-pointer select-none shrink-0">
              <input type="checkbox" className="accent-[var(--accent-primary)] cursor-pointer" checked={app.closeOnEnd} onChange={(e) => patch(i, { closeOnEnd: e.target.checked })} />
              Close when Game Mode ends
            </label>
          </div>
        </div>
      ))}
      {apps.length < 10 && (
        <div className="flex items-center flex-wrap gap-2">
          <div className="relative">
            <button
              onClick={() => setPicking((v) => !v)}
              className="h-8 px-3 rounded-md border border-[var(--border-subtle)] bg-[var(--bg-sidebar)] hover:border-[var(--border-strong)] text-[12px] text-[var(--text-primary)] flex items-center gap-1.5 cursor-pointer"
            >
              <Plus size={13} /> Add a running app
            </button>
            {picking && (
              <>
                <div className="fixed inset-0 z-[90]" onMouseDown={() => setPicking(false)} />
                <div className="absolute z-[91] left-0 top-9 w-72 max-h-64 overflow-y-auto rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)] shadow-2xl py-1">
                  {choices.length === 0 && <div className="px-3 py-4 text-center text-[11.5px] text-[var(--text-muted)]">Open the app first, then pick it here.</div>}
                  {choices.map((p) => (
                    <button
                      key={p.path}
                      onClick={() => {
                        add(p.path);
                        setPicking(false);
                      }}
                      title={p.path}
                      className="w-full text-left px-3 py-1.5 text-[12px] text-[var(--text-primary)] hover:bg-[var(--accent-soft)] cursor-pointer truncate flex items-center gap-2"
                    >
                      <AppMark name={p.name} path={p.path} size={16} />
                      <span className="truncate">{p.name}</span>
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>
          <input
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && add(typed)) setTyped("");
            }}
            placeholder="Or paste a path, like C:\Program Files\ExitLag\ExitLag.exe"
            className="flex-1 h-8 px-2 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] text-[11.5px] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] placeholder:text-[var(--text-muted)]"
          />
          <button
            disabled={!typedOk}
            onClick={() => add(typed) && setTyped("")}
            className="h-8 px-3 rounded-md border border-[var(--border-subtle)] text-[12px] text-[var(--text-primary)] enabled:hover:border-[var(--border-strong)] disabled:opacity-40 cursor-pointer disabled:cursor-default"
          >
            Add
          </button>
        </div>
      )}
      <p className="text-[11px] text-[var(--text-muted)] leading-snug">
        Apps already running are left alone. An app that needs admin rights, like ExitLag, shows one Windows prompt, and is skipped if a game is already full screen.
        Outplayed runs inside Overwolf: add Overwolf&apos;s launcher and use the arguments from its desktop shortcut.
      </p>
    </div>
  );
};
