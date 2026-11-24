import React, { useEffect, useMemo, useRef, useState } from "react";
import { BookmarkPlus, Check, LayoutTemplate, Replace, Trash2, X } from "lucide-react";
import { DeckIcon } from "./DeckIcon";
import {
  BUILT_IN_TEMPLATES,
  TEMPLATE_CATEGORIES,
  instantiateTemplate,
  loadCustomTemplates,
  saveCustomTemplates,
  templateFromPage,
  type PageTemplate,
} from "../data/pageTemplates";
import type { ProfilePage } from "../types/deckforge";

interface PageTemplatesProps {
  currentPage: ProfilePage | null;
  profileName: string;
  onAdd: (page: ProfilePage) => void;
  onReplace: (page: ProfilePage) => void;
  onClose: () => void;
  notify: (text: string, kind?: "success" | "info" | "warning") => void;
}

/** Ten key faces in the 5 x 2 layout of the deck. */
const KeyGrid: React.FC<{ page: ProfilePage; size: "sm" | "lg" }> = ({ page, size }) => {
  const keys = page.keys.slice(0, 10);
  const lg = size === "lg";
  return (
    <div className={`grid grid-cols-5 ${lg ? "gap-1.5" : "gap-1"}`}>
      {keys.map((k) => {
        const empty = k.actionId === "none";
        return (
          <div
            key={k.index}
            title={empty ? "Empty" : `${k.title} · ${k.actionLabel}`}
            className={`rounded-md border flex flex-col items-center justify-center overflow-hidden ${
              lg ? "aspect-square p-1 gap-0.5" : "aspect-square gap-0"
            }`}
            style={{
              background: empty ? "#10131D" : k.bgColor,
              borderColor: empty ? "#272D3D" : `${k.accentColor}66`,
            }}
          >
            <DeckIcon name={k.iconName} size={lg ? 18 : 11} color={empty ? "#4A5368" : k.accentColor} />
            {lg && (
              <span
                className="text-[9px] leading-none font-semibold truncate max-w-full"
                style={{ color: empty ? "#4A5368" : "#F4F5FA" }}
              >
                {empty ? "" : k.title}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
};

export const PageTemplates: React.FC<PageTemplatesProps> = ({
  currentPage,
  profileName,
  onAdd,
  onReplace,
  onClose,
  notify,
}) => {
  const [custom, setCustom] = useState<PageTemplate[]>(() => loadCustomTemplates());
  const [category, setCategory] = useState<string>("All");
  const [selectedId, setSelectedId] = useState<string>(BUILT_IN_TEMPLATES[0].id);
  const [pageName, setPageName] = useState<string>(BUILT_IN_TEMPLATES[0].name);
  const [saveName, setSaveName] = useState<string>("");
  const [confirmReplace, setConfirmReplace] = useState(false);
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;

  const all = useMemo(() => [...BUILT_IN_TEMPLATES, ...custom], [custom]);
  const visible = useMemo(
    () => all.filter((tpl) => category === "All" || tpl.category === category),
    [all, category]
  );
  const selected = all.find((tpl) => tpl.id === selectedId) ?? all[0];

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    dialogRef.current?.querySelector<HTMLElement>("button, input")?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeRef.current();
        return;
      }
      if (e.key !== "Tab") return;
      const focusable = Array.from(
        dialogRef.current?.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled)") ?? []
      );
      if (!focusable.length) return;
      const idx = focusable.indexOf(document.activeElement as HTMLElement);
      e.preventDefault();
      focusable[(idx + (e.shiftKey ? -1 : 1) + focusable.length) % focusable.length].focus();
    };
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      previous?.focus();
    };
  }, []);

  const choose = (tpl: PageTemplate) => {
    setSelectedId(tpl.id);
    setPageName(tpl.name);
    setConfirmReplace(false);
  };

  const countFor = (cat: string) =>
    cat === "All" ? all.length : all.filter((tpl) => tpl.category === cat).length;

  const assigned = selected.page.keys.slice(0, 10).filter((k) => k.actionId !== "none");

  const saveCurrent = () => {
    if (!currentPage) return;
    const tpl = templateFromPage(currentPage, saveName || currentPage.name);
    const next = [...custom, tpl];
    setCustom(next);
    saveCustomTemplates(next);
    setSaveName("");
    setCategory("Mine");
    choose(tpl);
    notify(`Saved "${tpl.name}" to My templates`, "success");
  };

  const removeCustom = (tpl: PageTemplate) => {
    const next = custom.filter((item) => item.id !== tpl.id);
    setCustom(next);
    saveCustomTemplates(next);
    choose(BUILT_IN_TEMPLATES[0]);
    notify(`Deleted "${tpl.name}"`, "info");
  };

  return (
    <div
      className="fixed inset-0 z-40 bg-black/65 backdrop-blur-[2px] flex items-center justify-center p-6 animate-fade-in"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="templates-title"
        className="w-[1040px] max-w-full h-[640px] max-h-full bg-[var(--bg-app)] border border-[var(--border-strong)] rounded-2xl shadow-2xl flex flex-col overflow-hidden animate-pop-in"
      >
        <div className="flex items-center gap-3 px-5 h-14 border-b border-[var(--border-subtle)] shrink-0">
          <div className="w-8 h-8 rounded-lg bg-[var(--accent-soft)] text-[var(--accent-text)] flex items-center justify-center">
            <LayoutTemplate size={16} />
          </div>
          <div className="flex-1 min-w-0">
            <div id="templates-title" className="text-sm font-semibold text-[var(--text-primary)]">Page templates</div>
            <div className="text-[11px] text-[var(--text-secondary)]">
              Start a page from a ready-made layout in {profileName}. Every key is a real action you can still edit.
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close templates"
            className="w-8 h-8 rounded-md text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-panel)] flex items-center justify-center cursor-pointer transition-colors"
          >
            <X size={16} />
          </button>
        </div>

        <div className="flex-1 min-h-0 grid grid-cols-[152px_minmax(0,1fr)_288px] xl:grid-cols-[168px_minmax(0,1fr)_316px]">
          {/* Categories */}
          <div className="border-r border-[var(--border-subtle)] p-3 flex flex-col gap-0.5 overflow-y-auto">
            {TEMPLATE_CATEGORIES.map((cat) => {
              const active = category === cat;
              const count = countFor(cat);
              return (
                <button
                  key={cat}
                  type="button"
                  aria-pressed={active}
                  onClick={() => setCategory(cat)}
                  className={`flex items-center justify-between text-left px-2.5 py-1.5 rounded-md text-[12px] cursor-pointer transition-colors ${
                    active
                      ? "bg-[var(--accent-soft)] text-[var(--text-primary)] font-semibold"
                      : "text-[var(--text-secondary)] hover:bg-[var(--bg-panel)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  <span>{cat === "Mine" ? "My templates" : cat}</span>
                  <span className={`text-[10px] font-mono tabular-nums ${active ? "text-[var(--accent-text)]" : "text-[var(--text-muted)]"}`}>{count}</span>
                </button>
              );
            })}
          </div>

          {/* Cards */}
          <div className="overflow-y-auto p-4">
            {visible.length === 0 ? (
              <div className="h-full flex flex-col items-center justify-center text-center text-[var(--text-muted)] gap-2">
                <BookmarkPlus size={26} />
                <div className="text-sm text-[var(--text-secondary)]">No saved templates yet</div>
                <div className="text-xs max-w-[260px]">
                  Build a page you like, open this window, and use “Save current page as template”.
                </div>
              </div>
            ) : (
              <div className="grid grid-cols-2 xl:grid-cols-3 gap-3">
                {visible.map((tpl) => {
                  const active = tpl.id === selected.id;
                  return (
                    <button
                      key={tpl.id}
                      type="button"
                      aria-pressed={active}
                      onClick={() => choose(tpl)}
                      onDoubleClick={() => onAdd(instantiateTemplate(tpl, tpl.name))}
                      className={`relative text-left rounded-xl border p-3 flex flex-col gap-2.5 cursor-pointer transition-all duration-150 ${
                        active
                          ? "border-[var(--accent-primary)] bg-[var(--accent-soft)] shadow-[0_0_0_1px_rgba(124,131,247,0.3)]"
                          : "border-[var(--border-subtle)] bg-[var(--bg-sidebar)] hover:border-[var(--border-strong)] hover:-translate-y-[1px] hover:shadow-md"
                      }`}
                    >
                      <KeyGrid page={tpl.page} size="sm" />
                      <div>
                        <div className="flex items-center gap-2">
                          <span
                            className="w-2 h-2 rounded-full shrink-0"
                            style={{ background: tpl.accent }}
                          />
                          <span className="text-[13px] font-semibold text-[var(--text-primary)] truncate flex-1">{tpl.name}</span>
                          {active && <Check size={13} className="shrink-0 text-[var(--accent-text)]" />}
                        </div>
                        <div className="text-[11px] leading-snug text-[var(--text-secondary)] mt-1 line-clamp-2">
                          {tpl.tagline}
                        </div>
                      </div>
                    </button>
                  );
                })}
              </div>
            )}
          </div>

          {/* Detail */}
          <div className="border-l border-[var(--border-subtle)] p-4 flex flex-col gap-3 overflow-y-auto bg-[var(--bg-sidebar)]">
            <div>
              <div className="text-[11px] font-semibold text-[var(--text-muted)]">{selected.category}</div>
              <div className="text-base font-semibold text-[var(--text-primary)]">{selected.name}</div>
              <div className="text-xs text-[var(--text-secondary)] mt-0.5">{selected.tagline}</div>
            </div>

            <KeyGrid page={selected.page} size="lg" />

            <div className="text-[11px] text-[var(--text-secondary)]">
              {assigned.length === 0 ? (
                "All ten keys are empty. Pick actions from the library below the deck."
              ) : (
                <>
                  <span className="text-[var(--text-primary)] font-medium">{assigned.length} keys</span> ·{" "}
                  {assigned
                    .slice(0, 6)
                    .map((k) => k.title)
                    .join(", ")}
                  {assigned.length > 6 ? ` and ${assigned.length - 6} more` : ""}
                </>
              )}
            </div>

            <label className="flex flex-col gap-1">
              <span className="text-[11px] text-[var(--text-secondary)]">Page name</span>
                <input
                value={pageName}
                onChange={(e) => setPageName(e.target.value)}
                className="h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none transition-colors"
              />
            </label>

            <div className="flex flex-col gap-2">
              <button
                type="button"
                onClick={() => onAdd(instantiateTemplate(selected, pageName))}
                className="h-9 rounded-md bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] active:scale-[0.98] text-white text-xs font-semibold flex items-center justify-center gap-1.5 cursor-pointer shadow-xs transition-all"
              >
                <Check size={14} /> Add as a new page
              </button>
              {!confirmReplace ? (
                <button
                  type="button"
                  onClick={() => setConfirmReplace(true)}
                  disabled={!currentPage}
                  className="h-9 rounded-md border border-[var(--border-subtle)] hover:border-[var(--accent-primary)] hover:bg-[var(--bg-panel)] text-[var(--text-primary)] text-xs font-medium flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-40 transition-colors"
                >
                  <Replace size={14} /> Replace “{currentPage?.name ?? "current page"}”
                </button>
              ) : (
                <div className="rounded-md border border-[var(--status-warning)]/45 bg-[var(--status-warning)]/10 p-2.5 flex flex-col gap-2 animate-pop-in">
                  <div className="text-[11px] text-[var(--status-warning)]">
                    This overwrites every key, touch zone and knob on “{currentPage?.name}”. You can undo it with Ctrl+Z.
                  </div>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      onClick={() => onReplace(instantiateTemplate(selected, pageName, currentPage?.id))}
                      className="flex-1 h-8 rounded-md bg-[var(--status-warning)] hover:brightness-110 active:scale-[0.98] text-[#1A140A] text-xs font-semibold cursor-pointer transition-all"
                    >
                      Replace page
                    </button>
                    <button
                      type="button"
                      onClick={() => setConfirmReplace(false)}
                      className="flex-1 h-8 rounded-md border border-[var(--border-subtle)] hover:bg-[var(--bg-panel)] text-[var(--text-primary)] text-xs cursor-pointer transition-colors"
                    >
                      Keep it
                    </button>
                  </div>
                </div>
              )}
              {!selected.builtIn && (
                <button
                  type="button"
                  onClick={() => removeCustom(selected)}
                  className="h-8 rounded-md border border-[var(--status-error)]/40 text-[var(--status-error)] hover:bg-[var(--status-error)]/10 text-xs flex items-center justify-center gap-1.5 cursor-pointer transition-colors"
                >
                  <Trash2 size={13} /> Delete this template
                </button>
              )}
            </div>

            <div className="mt-auto pt-3 border-t border-[var(--border-subtle)] flex flex-col gap-2">
              <div className="text-[11px] text-[var(--text-secondary)]">
                Save “{currentPage?.name ?? "current page"}” so you can reuse it in any profile.
              </div>
              <div className="flex gap-2">
                <input
                  value={saveName}
                  onChange={(e) => setSaveName(e.target.value)}
                  placeholder={currentPage?.name ?? "Template name"}
                  className="flex-1 min-w-0 h-8 px-2.5 rounded-md bg-[var(--bg-input)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none transition-colors"
                />
                <button
                  type="button"
                  onClick={saveCurrent}
                  disabled={!currentPage}
                  className="h-8 px-3 rounded-md bg-[var(--accent-soft)] border border-[var(--accent-primary)]/40 hover:bg-[var(--accent-primary)] text-[var(--accent-text)] hover:text-white text-xs font-medium flex items-center gap-1.5 cursor-pointer disabled:opacity-40 transition-colors"
                >
                  <BookmarkPlus size={13} /> Save
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
