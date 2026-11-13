import React from "react";
import { Plus, Trash2 } from "lucide-react";
import { deckforgeApi } from "../bridge/tauriBridge";
import type { AutoSwitchRule, Profile } from "../types/deckforge";

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

const appName = (exe: string) => {
  const known = KNOWN_APPS[exe.trim().toLowerCase()];
  if (known) return known;
  const base = exe.replace(/\.exe$/i, "").trim();
  return base ? base.charAt(0).toUpperCase() + base.slice(1) : "New app";
};

/** Stable hue per app, used for the letter tile when no real icon is available. */
const hueFor = (text: string) => [...text.toLowerCase()].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);

const iconCache = new Map<string, string | null>();

const AppIcon: React.FC<{ exe: string; size?: number }> = ({ exe, size = 32 }) => {
  const key = exe.trim().toLowerCase();
  const [src, setSrc] = React.useState<string | null>(iconCache.get(key) ?? null);
  React.useEffect(() => {
    let alive = true;
    if (!key) return;
    if (iconCache.get(key)) {
      setSrc(iconCache.get(key)!);
      return;
    }
    deckforgeApi
      .appIcon(key)
      .then((url) => {
        if (url) iconCache.set(key, url);
        if (alive) setSrc(url);
      })
      .catch(() => {});
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
    style={{ width: 30, height: 18, minWidth: 30, minHeight: 18, padding: 0, border: 0, borderRadius: 9 }}
    className={`relative shrink-0 inline-block transition-colors cursor-pointer ${
      checked ? "bg-[var(--accent-primary)]" : "bg-[var(--border-strong)]"
    }`}
  >
    <span
      style={{ position: "absolute", top: 2, left: checked ? 14 : 2, width: 14, height: 14, borderRadius: 7 }}
      className="bg-white transition-[left] duration-150"
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
}

/** Pages listed with the apps that open them. Drag an app onto another page to move it. */
export const AppRuleList: React.FC<Props> = ({ rules, profiles, saveRules }) => {
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [addingAt, setAddingAt] = React.useState<string | null>(null);
  const [draft, setDraft] = React.useState("");
  const [dropAt, setDropAt] = React.useState<string | null>(null);
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
    const processName = /\.exe$/i.test(exe) ? exe : `${exe}.exe`;
    saveRules([
      ...rules,
      {
        id: `rule-${Date.now()}`,
        processName,
        windowTitleContains: "",
        targetProfileId: profileId,
        targetPageId: pageId,
        priority: rules.length + 1,
        enabled: true,
      },
    ]);
  };

  const selected = rules.find((r) => r.id === selectedId) ?? null;

  return (
    <div className="flex flex-col gap-2">
      <div>
        <div className="text-[13px] font-semibold text-[var(--text-primary)]">App switching</div>
        <p className="text-[11.5px] leading-snug text-[var(--text-muted)] mt-0.5">
          A page opens when one of its apps is in front. Drag apps between pages.
        </p>
      </div>

      <datalist id="deckforge-known-apps">
        {Object.entries(KNOWN_APPS).map(([exe, name]) => (
          <option key={exe} value={exe}>
            {name}
          </option>
        ))}
      </datalist>

      <div className="flex flex-col gap-3">
        {profiles.map((prof) => (
          <div key={prof.id} className="flex flex-col">
            <div className="text-[11px] font-medium text-[var(--text-muted)] px-1 pb-1">{prof.name}</div>
            <div className="flex flex-col rounded-[var(--radius-md)] border border-[var(--border-subtle)] divide-y divide-[var(--border-subtle)]">
              {prof.pages.map((pg) => {
                const key = `${prof.id}:${pg.id}`;
                const apps = rules.filter((r) => r.targetProfileId === prof.id && pageOf(r)?.id === pg.id);
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
                    className={`px-2 py-1.5 flex flex-col gap-1.5 transition-colors ${dropAt === key ? "bg-[var(--bg-elevated)]" : ""}`}
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
                            title={`${appName(r.processName)}${r.enabled ? "" : " (off)"}`}
                            className={`rounded-[var(--radius-sm)] cursor-grab active:cursor-grabbing outline outline-2 outline-offset-1 ${
                              selectedId === r.id ? "outline-[var(--accent-primary)]" : "outline-transparent"
                            } ${r.enabled ? "" : "opacity-40 grayscale"}`}
                          >
                            <AppIcon exe={r.processName} size={24} />
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
                        <input
                          value={selected.windowTitleContains}
                          placeholder="Only when window title contains…"
                          onChange={(e) => patch(selected.id, { windowTitleContains: e.target.value })}
                          className={field}
                        />
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
  const apps = rules.filter((r) => r.targetProfileId === profile.id && pageOf(r)?.id === page.id);
  const selected = apps.find((r) => r.id === open);
  const patch = (id: string, change: Partial<AutoSwitchRule>) =>
    saveRules(rules.map((r) => (r.id === id ? { ...r, ...change } : r)));
  const add = () => {
    const exe = draft.trim();
    setDraft("");
    setOpen(null);
    if (!exe) return;
    const processName = /\.exe$/i.test(exe) ? exe : `${exe}.exe`;
    saveRules([
      ...rules,
      {
        id: `rule-${Date.now()}`,
        processName,
        windowTitleContains: "",
        targetProfileId: profile.id,
        targetPageId: page.id,
        priority: rules.length + 1,
        enabled: true,
      },
    ]);
  };

  return (
    <div ref={boxRef} className="relative flex items-center gap-1 h-8 pl-2.5 pr-1 rounded-[var(--radius-sm)] bg-[var(--bg-panel)] border border-[var(--border-subtle)]">
      <span className="text-[11px] text-[var(--text-muted)] mr-0.5">{apps.length ? "Opens with" : "No app"}</span>
      {apps.map((r) => (
        <button
          key={r.id}
          type="button"
          onClick={() => setOpen(open === r.id ? null : r.id)}
          title={`${appName(r.processName)}${r.enabled ? "" : " (off)"}`}
          className={`rounded-[4px] cursor-pointer outline outline-2 outline-offset-1 ${open === r.id ? "outline-[var(--accent-primary)]" : "outline-transparent"} ${
            r.enabled ? "" : "opacity-40 grayscale"
          }`}
        >
          <AppIcon exe={r.processName} size={20} />
        </button>
      ))}
      <button
        type="button"
        onClick={() => {
          setDraft("");
          setOpen(open === "add" ? null : "add");
        }}
        title={`Add an app that opens ${page.name}`}
        className="w-5 h-5 rounded-[4px] flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer"
      >
        <Plus size={12} />
      </button>

      {open && (
        <div className="absolute bottom-full mb-2 left-0 z-30 w-60 p-2.5 rounded-[var(--radius-md)] bg-[var(--bg-elevated)] border border-[var(--border-subtle)] shadow-lg flex flex-col gap-2">
          {open === "add" ? (
            <>
              <div className="text-[12px] font-medium text-[var(--text-primary)]">Open {page.name} when this app is in front</div>
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
                placeholder="Program, e.g. obs64.exe"
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") add();
                  if (e.key === "Escape") setOpen(null);
                }}
                className={`${field} font-mono`}
              />
              <button
                type="button"
                onClick={add}
                className="h-7 rounded-[var(--radius-sm)] bg-[var(--accent-primary)] text-white text-[12px] font-medium cursor-pointer"
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
              <input
                value={selected.windowTitleContains}
                placeholder="Only when window title contains…"
                onChange={(e) => patch(selected.id, { windowTitleContains: e.target.value })}
                className={field}
              />
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
