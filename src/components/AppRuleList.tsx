import React from "react";
import { Gamepad2, Plus, Trash2 } from "lucide-react";
import { deckforgeApi } from "../bridge/tauriBridge";
import type { AutoSwitchRule, Profile } from "../types/deckforge";
import { assignUnrestrictedApp, conflictingRule } from "../lib/appRuleConflicts";
import { createAppIconLoader } from "../lib/appIconCache";
import { previewAppSwitch } from "../lib/appSwitchPreview";

/** Friendly names for common apps, so a rule reads "OBS Studio" rather than "obs64.exe". */
const KNOWN_APPS: Record<string, string> = {
  "obs64.exe": "OBS Studio",
  "obs32.exe": "OBS Studio",
  "resolve.exe": "DaVinci Resolve",
  "code.exe": "Visual Studio Code",
  "spotify.exe": "Spotify",
  "discord.exe": "Discord",
  "chrome.exe": "Google Chrome",
  "msedge.exe": "Microsoft Edge",
  "firefox.exe": "Firefox",
  "photoshop.exe": "Photoshop",
  "premiere pro.exe": "Premiere Pro",
  "afterfx.exe": "After Effects",
  "blender.exe": "Blender",
  "steam.exe": "Steam",
  "slack.exe": "Slack",
  "teams.exe": "Microsoft Teams",
  "ms-teams.exe": "Microsoft Teams",
  "zoom.exe": "Zoom",
};

/** Rule that matches any full-screen game rather than one program. */
export const ANY_GAME = "@game";
const isAnyGame = (exe: string) => exe.trim().toLowerCase() === ANY_GAME;

const appName = (exe: string) => {
  if (isAnyGame(exe)) return "Any game";
  const known = KNOWN_APPS[exe.trim().toLowerCase()];
  if (known) return known;
  const base = exe.replace(/\.exe$/i, "").trim();
  return base ? base.charAt(0).toUpperCase() + base.slice(1) : "New app";
};

/** Stable hue per app, used for the letter tile when no real icon is available. */
const hueFor = (text: string) => [...text.toLowerCase()].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);

const loadAppIcon = createAppIconLoader(exe => deckforgeApi.appIcon(exe));

/** Programs with an open window, loaded while `active` is true. */
const useOpenApps = (active: boolean) => {
  const [apps, setApps] = React.useState<string[]>([]);
  React.useEffect(() => {
    if (!active) return;
    let alive = true;
    deckforgeApi.openApps().then((list) => alive && setApps(list));
    return () => {
      alive = false;
    };
  }, [active]);
  return apps;
};

const AppIcon: React.FC<{ exe: string; size?: number }> = ({ exe, size = 32 }) => {
  if (isAnyGame(exe)) {
    return (
      <div
        className="shrink-0 rounded-[var(--radius-sm)] flex items-center justify-center bg-[var(--accent-soft)] text-[var(--accent-primary)]"
        style={{ width: size, height: size }}
        aria-hidden
      >
        <Gamepad2 size={Math.round(size * 0.66)} />
      </div>
    );
  }
  return <ProgramIcon exe={exe} size={size} />;
};

const ProgramIcon: React.FC<{ exe: string; size?: number }> = ({ exe, size = 32 }) => {
  const key = exe.trim().toLowerCase();
  const [src, setSrc] = React.useState<string | null>(null);
  React.useEffect(() => {
    let alive = true;
    setSrc(null);
    loadAppIcon(key).then(url => { if (alive) setSrc(url); }).catch(() => {});
    return () => {
      alive = false;
    };
  }, [key]);
  const name = appName(exe);
  const hue = hueFor(name);
  return (
    <div
      className="shrink-0 rounded-[var(--radius-sm)] flex items-center justify-center overflow-hidden"
      style={{
        width: size,
        height: size,
        background: src ? "var(--bg-elevated)" : `hsl(${hue} 45% 22%)`,
        color: `hsl(${hue} 80% 82%)`,
      }}
      aria-hidden
    >
      {src ? (
        <img src={src} alt="" className="w-[78%] h-[78%] object-contain" draggable={false} />
      ) : (
        <span className="text-[13px] font-semibold">{name.charAt(0)}</span>
      )}
    </div>
  );
};

const Switch: React.FC<{ checked: boolean; onChange: (v: boolean) => void; label: string }> = ({ checked, onChange, label }) => (
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={label}
    onClick={() => onChange(!checked)}
    className={`w-8 h-[18px] rounded-full p-0.5 transition-colors cursor-pointer shrink-0 ${
      checked ? "bg-[var(--accent-primary)]" : "bg-[var(--border-strong)]"
    }`}
  >
    <div
      className={`w-3.5 h-3.5 rounded-full bg-white shadow-xs transition-transform ${
        checked ? "translate-x-3.5" : "translate-x-0"
      }`}
    />
  </button>
);

const field =
  "h-7 w-full px-2 rounded-[var(--radius-sm)] bg-[var(--bg-input)] border border-[var(--border-subtle)] text-[12px] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)]";

const RULE_DRAG_TYPE = "application/x-deckforge-rule";

interface Props {
  rules: AutoSwitchRule[];
  profiles: Profile[];
  saveRules: (rules: AutoSwitchRule[]) => void;
  /** Only list this profile's pages; the rest are still used to find rules with no home. */
  showProfileId?: string;
  currentPageIndex?: number;
  /** Hide the section title when a parent already names this block. */
  embedded?: boolean;
}

/** Pages listed with the apps that open them. Drag an app onto another page to move it. */
export const AppRuleList: React.FC<Props> = ({ rules, profiles, saveRules, showProfileId, currentPageIndex = 0, embedded = false }) => {
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [addingAt, setAddingAt] = React.useState<string | null>(null);
  const [draft, setDraft] = React.useState("");
  const [dropAt, setDropAt] = React.useState<string | null>(null);
  const [profileDraft, setProfileDraft] = React.useState("");
  const [addingProfile, setAddingProfile] = React.useState<string | null>(null);
  const [previewExe, setPreviewExe] = React.useState("");
  const [previewTitle, setPreviewTitle] = React.useState("");
  const [previewGame, setPreviewGame] = React.useState(false);
  const [previewProfileId, setPreviewProfileId] = React.useState<string | null>(null);
  const activePreviewProfileId = previewProfileId && profiles.some(p => p.id === previewProfileId) ? previewProfileId : (showProfileId ?? profiles[0]?.id ?? "");
  const previewPageIndex = activePreviewProfileId === showProfileId ? currentPageIndex : 0;
  const preview = previewExe.trim() || previewGame
    ? previewAppSwitch(rules, profiles, activePreviewProfileId, previewPageIndex, previewExe, previewTitle, previewGame)
    : null;
  const previewProfile = profiles.find(p => p.id === preview?.profileId);
  const previewRule = rules.find(r => r.id === preview?.ruleId);
  const previewPageRule = rules.find(r => r.id === preview?.pageRuleId);
  const patch = (id: string, change: Partial<AutoSwitchRule>) =>
    saveRules(rules.map((r) => (r.id === id ? { ...r, ...change } : r)));

  /** The page a rule opens: its own page if it still exists, else the profile's first. */
  const pageOf = (rule: AutoSwitchRule) => {
    const prof = profiles.find((p) => p.id === rule.targetProfileId);
    if (!prof) return null;
    return prof.pages.find((pg) => pg.id === rule.targetPageId) ?? prof.pages[0] ?? null;
  };

  const addApp = (profileId: string, pageId: string) => {
    const exe = draft.trim();
    setAddingAt(null);
    setDraft("");
    if (!exe) return;
    const processName = isAnyGame(exe) || /\.exe$/i.test(exe) ? exe : `${exe}.exe`;
    saveRules(assignUnrestrictedApp(rules, processName, profileId, pageId));
  };

  const addProfileApp = (profileId: string) => {
    const exe = profileDraft.trim();
    if (!exe) return;
    const processName = isAnyGame(exe) || /\.exe$/i.test(exe) ? exe : `${exe}.exe`;
    saveRules(assignUnrestrictedApp(rules, processName, profileId, ""));
    setProfileDraft(""); setAddingProfile(null);
  };
  const selected = rules.find((r) => r.id === selectedId) ?? null;
  const shown = showProfileId ? profiles.filter((p) => p.id === showProfileId) : profiles;
  // Rules whose profile is gone would otherwise vanish from view while still saved.
  const homeless = rules.filter((r) => !profiles.some((p) => p.id === r.targetProfileId));
  const conflict = (r: AutoSwitchRule) => {
    const winner = conflictingRule(rules.filter(other => Boolean(other.targetPageId) === Boolean(r.targetPageId)), r);
    return winner && winner.id !== r.id ? winner : null;
  };
  const isStale = (r: AutoSwitchRule) =>
    !!r.targetPageId && !profiles.find((p) => p.id === r.targetProfileId)?.pages.some((pg) => pg.id === r.targetPageId);

  return (
    <div className="flex flex-col gap-2.5">
      {embedded ? (
        <p className="text-[11px] leading-snug text-[var(--text-muted)]">
          Profile rules work anywhere. Page rules work only while that profile is active.
        </p>
      ) : (
      <div>
        <div className="text-[11.5px] font-semibold tracking-wide text-[var(--text-secondary)]">App switching</div>
        <p className="text-[11px] leading-snug text-[var(--text-muted)] mt-0.5">
          Profile rules work anywhere. Page rules work only while that profile is active.
        </p>
      </div>
      )}

      <details className="rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-panel)]/70 p-2 text-xs">
        <summary className="cursor-pointer select-none text-[11.5px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)]">Preview a switch</summary>
        <div className="mt-2.5 flex flex-col gap-2">
          <label className="flex flex-col gap-1 text-[11px] text-[var(--text-secondary)]">Starting profile
            <select aria-label="Preview starting profile" className={field} value={activePreviewProfileId} onChange={e => setPreviewProfileId(e.target.value)}>
              {profiles.map(profile => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
            </select>
          </label>
          <input aria-label="Preview app executable" className={field} value={previewExe} onChange={e => setPreviewExe(e.target.value)} placeholder="App, e.g. spotify.exe" />
          <input aria-label="Preview window title" className={field} value={previewTitle} onChange={e => setPreviewTitle(e.target.value)} placeholder="Window title (optional)" />
          <label className="flex items-center gap-2 text-[11.5px] text-[var(--text-secondary)] cursor-pointer"><input type="checkbox" className="accent-[var(--accent-primary)]" checked={previewGame} onChange={e => setPreviewGame(e.target.checked)} /> Treat as a game</label>
          {preview && <div role="status" className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-input)] p-2 leading-relaxed">
            <strong className="block text-[var(--text-primary)]">{preview.switches ? `Opens ${previewProfile?.name ?? preview.profileId} / ${previewProfile?.pages[preview.pageIndex]?.name ?? "page 1"}` : "Stays on the current page"}</strong>
            <span>{preview.reason}</span>
            {previewRule && <span className="block mt-1 text-[var(--text-muted)]">Winning rule: {previewRule.processName}{previewRule.windowTitleContains ? ` · title contains “${previewRule.windowTitleContains}”` : ""}{previewPageRule ? ` · page rule: ${previewPageRule.processName}` : ""}</span>}
          </div>}
          <p className="text-[10.5px] text-[var(--text-muted)] leading-snug">Preview only. It does not change the deck. Game detection and the current foreground window are simulated.</p>
        </div>
      </details>

      <datalist id="deckforge-known-apps">
        <option value={ANY_GAME}>Any game</option>
        {Object.entries(KNOWN_APPS).map(([exe, name]) => (
          <option key={exe} value={exe}>
            {name}
          </option>
        ))}
      </datalist>

      <div className="flex flex-col gap-3">
        {shown.map((prof) => (
          <div key={prof.id} className="flex flex-col gap-2">
            <div className="text-[11px] font-medium text-[var(--text-muted)] px-1">{prof.name}</div>
            <div className="rounded-[var(--radius-md)] border border-[var(--border-subtle)] bg-[var(--bg-panel)]/45 p-2.5">
              <div className="text-[11px] font-semibold text-[var(--text-primary)]">Switch to this profile</div>
              <div className="text-[10.5px] text-[var(--text-muted)] mt-0.5">These apps switch profiles from anywhere.</div>
              <div className="flex flex-wrap items-center gap-1.5 mt-2">
                {rules.filter(r => r.targetProfileId === prof.id && !r.targetPageId).map(r => (
                  <button
                    key={r.id}
                    type="button"
                    onClick={() => setSelectedId(selectedId === r.id ? null : r.id)}
                    className={`rounded-md border px-2 py-1 text-[11px] font-medium cursor-pointer transition-colors ${
                      selectedId === r.id
                        ? "bg-[var(--accent-soft)] border-[var(--accent-primary)]/60 text-[var(--text-primary)]"
                        : "bg-[var(--bg-input)] border-[var(--border-subtle)] text-[var(--text-secondary)] hover:border-[var(--border-strong)] hover:text-[var(--text-primary)]"
                    } ${r.enabled ? "" : "opacity-50"}`}
                    title={`${appName(r.processName)}${r.enabled ? "" : " (off)"}`}
                  >
                    {appName(r.processName)}{r.enabled ? "" : " (off)"}
                  </button>
                ))}
                <button
                  type="button"
                  onClick={() => setAddingProfile(addingProfile === prof.id ? null : prof.id)}
                  className="rounded-md border border-dashed border-[var(--border-strong)] px-2 py-1 text-[11px] text-[var(--text-secondary)] hover:border-[var(--accent-primary)] hover:text-[var(--text-primary)] cursor-pointer transition-colors"
                >
                  + App
                </button>
              </div>
              {addingProfile === prof.id && (
                <div className="flex gap-1.5 mt-2">
                  <input
                    aria-label="App for profile switching"
                    list="deckforge-known-apps"
                    value={profileDraft}
                    onChange={e => setProfileDraft(e.target.value)}
                    onKeyDown={e => { if (e.key === "Enter") addProfileApp(prof.id); if (e.key === "Escape") setAddingProfile(null); }}
                    placeholder="Program, e.g. spotify.exe"
                    className={field}
                  />
                  <button
                    type="button"
                    onClick={() => addProfileApp(prof.id)}
                    className="h-7 px-2.5 rounded-[var(--radius-sm)] bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white text-[11px] font-semibold cursor-pointer shrink-0 transition-colors"
                  >
                    Add
                  </button>
                </div>
              )}
              {selected && selected.targetProfileId === prof.id && !selected.targetPageId && (
                <div className="mt-2 flex flex-col gap-1.5 rounded-[var(--radius-sm)] bg-[var(--bg-panel)] border border-[var(--border-subtle)] p-2">
                  <div className="flex items-center gap-2">
                    <AppIcon exe={selected.processName} size={20} />
                    <span className="text-[12px] font-medium text-[var(--text-primary)] truncate flex-1">{appName(selected.processName)}</span>
                    <Switch checked={selected.enabled} onChange={enabled => patch(selected.id, {enabled})} label={`Switch to ${prof.name} for ${appName(selected.processName)}`}/>
                  </div>
                  <input aria-label="Only when window title contains" value={selected.windowTitleContains} placeholder="Window title (optional)" onChange={e => patch(selected.id, {windowTitleContains:e.target.value})} className={field}/>
                  <button type="button" onClick={() => { saveRules(rules.filter(r => r.id !== selected.id)); setSelectedId(null); }} className="self-start flex items-center gap-1.5 text-[11.5px] text-[var(--status-error)] hover:underline cursor-pointer">
                    <Trash2 size={12} /> Remove
                  </button>
                </div>
              )}
            </div>
            <div className="text-[11px] font-semibold text-[var(--text-primary)] px-1">Switch pages within {prof.name}</div>
            <div className="flex flex-col rounded-[var(--radius-md)] border border-[var(--border-subtle)] bg-[var(--bg-panel)]/35 divide-y divide-[var(--border-subtle)]">
              {prof.pages.map((pg) => {
                const key = `${prof.id}:${pg.id}`;
                const apps = rules.filter((r) => r.targetProfileId === prof.id && !!r.targetPageId && pageOf(r)?.id === pg.id);
                return (
                  <div
                    key={pg.id}
                    onDragOver={(e) => {
                      if (!e.dataTransfer.types.includes(RULE_DRAG_TYPE)) return;
                      e.preventDefault();
                      if (dropAt !== key) setDropAt(key);
                    }}
                    onDragLeave={() => setDropAt((d) => (d === key ? null : d))}
                    onDrop={(e) => {
                      const id = e.dataTransfer.getData(RULE_DRAG_TYPE);
                      setDropAt(null);
                      if (id) patch(id, { targetProfileId: prof.id, targetPageId: pg.id });
                    }}
                    className={`px-2.5 py-2 flex flex-col gap-1.5 transition-colors ${dropAt === key ? "bg-[var(--bg-elevated)]" : ""}`}
                  >
                    <div className="flex items-center gap-2 min-h-[26px]">
                      <span className="text-[12.5px] text-[var(--text-primary)] truncate min-w-0 flex-1">{pg.name}</span>
                      <div className="flex items-center gap-1 flex-wrap justify-end">
                        {apps.map((r) => (
                          <button
                            key={r.id}
                            type="button"
                            draggable
                            onDragStart={(e) => {
                              e.dataTransfer.setData(RULE_DRAG_TYPE, r.id);
                              e.dataTransfer.effectAllowed = "move";
                            }}
                            onClick={() => setSelectedId(selectedId === r.id ? null : r.id)}
                            title={conflict(r) ? `${appName(r.processName)}: another enabled rule wins. Move this app to resolve the duplicate.` : isStale(r) ? `${appName(r.processName)}: its page was deleted, so it opens page 1. Drag it to a page to fix.` : `${appName(r.processName)}${r.enabled ? "" : " (off)"}`}
                            className={`rounded-[var(--radius-sm)] cursor-grab active:cursor-grabbing transition-all ${
                              selectedId === r.id ? "ring-2 ring-[var(--accent-primary)] ring-offset-1 ring-offset-[var(--bg-sidebar)]" : "hover:brightness-110"
                            } ${r.enabled ? "" : "opacity-40 grayscale"}`}
                          >
                            <span className="relative block">
                              <AppIcon exe={r.processName} size={24} />
                              {(isStale(r) || conflict(r)) && (
                                <span className="absolute -top-1 -right-1 w-2 h-2 rounded-full bg-[var(--status-warning)] ring-2 ring-[var(--bg-sidebar)]" />
                              )}
                            </span>
                          </button>
                        ))}
                        <button
                          type="button"
                          onClick={() => {
                            setAddingAt(addingAt === key ? null : key);
                            setDraft("");
                          }}
                          title={`Add an app that opens ${pg.name}`}
                          className="w-6 h-6 rounded-[var(--radius-sm)] border border-dashed border-[var(--border-strong)] flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:border-[var(--accent-primary)] cursor-pointer"
                        >
                          <Plus size={13} />
                        </button>
                      </div>
                    </div>
                    {addingAt === key && (
                      <input
                        autoFocus
                        list="deckforge-known-apps"
                        value={draft}
                        placeholder="Program, e.g. obs64.exe"
                        onChange={(e) => setDraft(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") addApp(prof.id, pg.id);
                          if (e.key === "Escape") setAddingAt(null);
                        }}
                        onBlur={() => addApp(prof.id, pg.id)}
                        className={`${field} font-mono`}
                      />
                    )}
                    {selected && apps.some((r) => r.id === selected.id) && (
                      <div className="flex flex-col gap-1.5 rounded-[var(--radius-sm)] bg-[var(--bg-panel)] p-2">
                        <div className="flex items-center gap-2">
                          <AppIcon exe={selected.processName} size={20} />
                          <span className="text-[12px] font-medium text-[var(--text-primary)] truncate flex-1">
                            {appName(selected.processName)}
                          </span>
                          <Switch
                            checked={selected.enabled}
                            onChange={(enabled) => patch(selected.id, { enabled })}
                            label={`Switch pages for ${appName(selected.processName)}`}
                          />
                        </div>
                        {isAnyGame(selected.processName) ? (
                          <p className="text-[11px] text-[var(--text-muted)] leading-snug">Opens for any full-screen game. A game added on its own page still wins.</p>
                        ) : <input
                          value={selected.windowTitleContains}
                          placeholder="Only when window title contains…"
                          onChange={(e) => patch(selected.id, { windowTitleContains: e.target.value })}
                          className={field}
                        />}
                        {conflict(selected) && <p role="status" className="text-[11px] text-[var(--status-warning)]">Another {appName(selected.processName)} rule has higher priority and opens {profiles.find(p => p.id === conflict(selected)?.targetProfileId)?.name ?? "another profile"}. Move this app here to resolve it.</p>}
                        <button
                          type="button"
                          onClick={() => {
                            saveRules(rules.filter((r) => r.id !== selected.id));
                            setSelectedId(null);
                          }}
                          className="self-start flex items-center gap-1.5 text-[11.5px] text-[var(--status-error)] hover:underline cursor-pointer"
                        >
                          <Trash2 size={12} /> Remove
                        </button>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        ))}
        {homeless.length > 0 && (
          <div className="flex flex-col gap-1 rounded-[var(--radius-md)] border border-dashed border-[var(--status-warning)]/50 p-2">
            <div className="text-[11.5px] text-[var(--status-warning)]">Profile deleted. Drag these onto a page, or remove them.</div>
            <div className="flex items-center gap-1 flex-wrap">
              {homeless.map((r) => (
                <span key={r.id} className="flex items-center gap-1">
                  <button
                    type="button"
                    draggable
                    onDragStart={(e) => {
                      e.dataTransfer.setData(RULE_DRAG_TYPE, r.id);
                      e.dataTransfer.effectAllowed = "move";
                    }}
                    title={appName(r.processName)}
                    className="cursor-grab active:cursor-grabbing"
                  >
                    <AppIcon exe={r.processName} size={24} />
                  </button>
                  <button
                    type="button"
                    onClick={() => saveRules(rules.filter((x) => x.id !== r.id))}
                    title={`Remove ${appName(r.processName)}`}
                    className="text-[var(--text-muted)] hover:text-[var(--status-error)] cursor-pointer"
                  >
                    <Trash2 size={12} />
                  </button>
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

/** The apps that open one page, shown beside the page picker. Click an app to change it; + adds one. */
export const PageApps: React.FC<{
  rules: AutoSwitchRule[];
  profile: Profile;
  pageIndex: number;
  saveRules: (rules: AutoSwitchRule[]) => void;
}> = ({ rules, profile, pageIndex, saveRules }) => {
  const [open, setOpen] = React.useState<string | null>(null); // rule id, "add", or null
  const [draft, setDraft] = React.useState("");
  const boxRef = React.useRef<HTMLDivElement>(null);
  const openApps = useOpenApps(open === "add");
  const page = profile.pages[pageIndex];
  React.useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!boxRef.current?.contains(e.target as Node)) setOpen(null);
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [open]);
  React.useEffect(() => setOpen(null), [profile.id, pageIndex]);
  if (!page) return null;

  const pageOf = (r: AutoSwitchRule) => profile.pages.find((pg) => pg.id === r.targetPageId) ?? profile.pages[0];
  const apps = rules.filter((r) => r.targetProfileId === profile.id && !!r.targetPageId && pageOf(r)?.id === page.id);
  const selected = apps.find((r) => r.id === open);
  const patch = (id: string, change: Partial<AutoSwitchRule>) =>
    saveRules(rules.map((r) => (r.id === id ? { ...r, ...change } : r)));
  const taken = new Set(apps.map((r) => r.processName.toLowerCase()));
  const choices = openApps.filter((exe) => !taken.has(exe));
  const add = (pick?: string) => {
    const exe = (pick ?? draft).trim();
    setDraft("");
    setOpen(null);
    if (!exe) return;
    const processName = isAnyGame(exe) || /\.exe$/i.test(exe) ? exe : `${exe}.exe`;
    saveRules(assignUnrestrictedApp(rules, processName, profile.id, page.id));
  };

  return (
    <div ref={boxRef} className="relative flex items-center gap-1 h-8 pl-2.5 pr-1 rounded-[var(--radius-sm)] bg-[var(--bg-panel)] border border-[var(--border-subtle)]">
      <span className="text-[11px] text-[var(--text-muted)] mr-0.5">{apps.length ? "Opens with" : "No app"}</span>
      {apps.map((r) => (
        <button
          key={r.id}
          type="button"
          onClick={() => setOpen(open === r.id ? null : r.id)}
          title={r.targetPageId && r.targetPageId !== page.id ? `${appName(r.processName)}: its page was deleted, so it opens page 1. Drag it to a page to fix.` : `${appName(r.processName)}${r.enabled ? "" : " (off)"}`}
          className={`relative rounded-[4px] cursor-pointer transition-all ${open === r.id ? "ring-2 ring-[var(--accent-primary)] ring-offset-1 ring-offset-[var(--bg-panel)]" : "hover:brightness-110"} ${
            r.enabled ? "" : "opacity-40 grayscale"
          }`}
        >
          <AppIcon exe={r.processName} size={20} />
          {r.targetPageId && r.targetPageId !== page.id && (
            <span className="absolute -top-1 -right-1 w-2 h-2 rounded-full bg-[var(--status-warning)] ring-2 ring-[var(--bg-panel)]" />
          )}
        </button>
      ))}
      <button
        type="button"
        onClick={() => {
          setDraft("");
          setOpen(open === "add" ? null : "add");
        }}
        title={`Add an app that opens ${page.name}`}
        className="w-5 h-5 rounded-[4px] flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
      >
        <Plus size={12} />
      </button>

      {open && (
        <div className="absolute bottom-full mb-2 left-0 z-30 w-64 p-3 rounded-[var(--radius-md)] bg-[var(--bg-elevated)] border border-[var(--border-strong)] shadow-2xl flex flex-col gap-2 animate-pop-in">
          {open === "add" ? (
            <>
              <div className="text-[12px] font-medium text-[var(--text-primary)]">Open {page.name} when this app is in front</div>
              {!taken.has(ANY_GAME) && (
                <button
                  type="button"
                  onClick={() => add(ANY_GAME)}
                  title="Any full-screen game, or a program in a Steam, Epic, Xbox or other game folder. An app added on its own still wins."
                  className="flex items-center gap-2 px-1 py-1 -mx-1 rounded-[var(--radius-sm)] hover:bg-[var(--bg-panel)] text-left cursor-pointer"
                >
                  <AppIcon exe={ANY_GAME} size={20} />
                  <span className="flex flex-col min-w-0">
                    <span className="text-[12px] text-[var(--text-primary)]">Any game</span>
                    <span className="text-[10.5px] text-[var(--text-muted)] truncate">Any full-screen game you play</span>
                  </span>
                </button>
              )}
              {choices.length > 0 && (
                <div className="flex flex-col gap-0.5 max-h-48 overflow-y-auto -mx-1">
                  <div className="text-[11px] text-[var(--text-muted)] px-1">Open now</div>
                  {choices.map((exe) => (
                    <button
                      key={exe}
                      type="button"
                      onClick={() => add(exe)}
                      className="flex items-center gap-2 px-1 py-1 rounded-[var(--radius-sm)] hover:bg-[var(--bg-panel)] text-left cursor-pointer"
                    >
                      <AppIcon exe={exe} size={20} />
                      <span className="text-[12px] text-[var(--text-primary)] truncate">{appName(exe)}</span>
                    </button>
                  ))}
                </div>
              )}
              <datalist id="deckforge-page-apps">
                {Object.entries(KNOWN_APPS).map(([exe, name]) => (
                  <option key={exe} value={exe}>
                    {name}
                  </option>
                ))}
              </datalist>
              <input
                autoFocus
                list="deckforge-page-apps"
                value={draft}
                placeholder={choices.length ? "Or type a program, e.g. obs64.exe" : "Program, e.g. obs64.exe"}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") add();
                  if (e.key === "Escape") setOpen(null);
                }}
                className={`${field} font-mono`}
              />
              <button
                type="button"
                onClick={() => add()}
                className="h-7 rounded-[var(--radius-sm)] bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white text-[12px] font-semibold cursor-pointer transition-colors"
              >
                Add app
              </button>
            </>
          ) : selected ? (
            <>
              <div className="flex items-center gap-2">
                <AppIcon exe={selected.processName} size={22} />
                <span className="text-[12.5px] font-medium text-[var(--text-primary)] truncate flex-1">{appName(selected.processName)}</span>
                <Switch
                  checked={selected.enabled}
                  onChange={(enabled) => patch(selected.id, { enabled })}
                  label={`Open ${page.name} for ${appName(selected.processName)}`}
                />
              </div>
              {isAnyGame(selected.processName) ? (
                <p className="text-[11px] text-[var(--text-muted)] leading-snug">Opens for any full-screen game. A game added on its own page still wins.</p>
              ) : <input
                value={selected.windowTitleContains}
                placeholder="Only when window title contains…"
                onChange={(e) => patch(selected.id, { windowTitleContains: e.target.value })}
                className={field}
              />}
              <button
                type="button"
                onClick={() => {
                  saveRules(rules.filter((r) => r.id !== selected.id));
                  setOpen(null);
                }}
                className="self-start flex items-center gap-1.5 text-[11.5px] text-[var(--status-error)] hover:underline cursor-pointer"
              >
                <Trash2 size={12} /> Remove from this page
              </button>
            </>
          ) : null}
        </div>
      )}
    </div>
  );
};
