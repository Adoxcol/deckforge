import React from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, Search, X } from "lucide-react";
import { DeckIcon } from "./DeckIcon";
import { groupOf, matchesQuery, nameOf } from "../lib/actionSearch";

export interface PickerChoice {
  id: string;
  actionLabel: string;
  title?: string;
  category?: string;
  description?: string;
  iconName?: string;
}

const PANEL_HEIGHT = 340;

export const ActionPicker: React.FC<{
  value: string;
  choices: PickerChoice[];
  onChange: (id: string) => void;
  onOpen?: () => void;
}> = ({ value, choices, onChange, onOpen }) => {
  const [open, setOpen] = React.useState(false);
  const [query, setQuery] = React.useState("");
  const [cursor, setCursor] = React.useState(0);
  const [rect, setRect] = React.useState<{ left: number; top: number; width: number } | null>(null);
  const buttonRef = React.useRef<HTMLButtonElement>(null);
  const listRef = React.useRef<HTMLDivElement>(null);
  const pickerId = React.useId();

  const current = choices.find((choice) => choice.id === value);
  const filtered = React.useMemo(() => choices.filter((choice) => matchesQuery(choice, query)), [choices, query]);
  const groups = React.useMemo(() => {
    const map = new Map<string, PickerChoice[]>();
    for (const choice of filtered) {
      const key = groupOf(choice);
      map.set(key, [...(map.get(key) ?? []), choice]);
    }
    return [...map.entries()];
  }, [filtered]);
  const flat = groups.flatMap(([, items]) => items);

  const place = () => {
    const box = buttonRef.current?.getBoundingClientRect();
    if (!box) return;
    const below = window.innerHeight - box.bottom;
    const top = below >= PANEL_HEIGHT + 8 ? box.bottom + 4 : Math.max(8, box.top - PANEL_HEIGHT - 4);
    setRect({ left: box.left, top, width: Math.max(box.width, 280) });
  };

  const openPicker = () => {
    place();
    setQuery("");
    setOpen(true);
    onOpen?.();
  };

  React.useEffect(() => {
    if (!open) return;
    const index = flat.findIndex((choice) => choice.id === value);
    setCursor(query ? 0 : Math.max(0, index));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, query]);

  React.useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    window.addEventListener("resize", close);
    return () => window.removeEventListener("resize", close);
  }, [open]);

  React.useEffect(() => {
    listRef.current?.querySelector(`[data-index="${cursor}"]`)?.scrollIntoView({ block: "nearest" });
  }, [cursor, open]);

  const choose = (choice: PickerChoice | undefined) => {
    if (!choice) return;
    onChange(choice.id);
    setOpen(false);
    buttonRef.current?.focus();
  };

  const onKey = (event: React.KeyboardEvent) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setCursor((c) => Math.min(flat.length - 1, c + 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setCursor((c) => Math.max(0, c - 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(flat[cursor]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
      buttonRef.current?.focus();
    }
  };

  let index = -1;
  return (
    <>
      <button
        ref={buttonRef}
        type="button"
        aria-expanded={open}
        aria-haspopup="listbox"
        onClick={() => (open ? setOpen(false) : openPicker())}
        onKeyDown={(event) => {
          if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            openPicker();
          } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
            openPicker();
            setQuery(event.key);
            event.preventDefault();
          }
        }}
        className="w-full h-8 pl-2 pr-1.5 rounded-md bg-[var(--bg-app)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] focus-visible:border-[var(--accent-primary)] text-xs text-[var(--text-primary)] outline-none flex items-center gap-2 cursor-pointer transition-colors"
      >
        {current?.iconName && <DeckIcon name={current.iconName} size={13} className="shrink-0 text-[var(--text-secondary)]" />}
        <span className="truncate text-left flex-1">{current ? current.actionLabel : value || "Choose an action"}</span>
        <ChevronDown size={14} className={`shrink-0 text-[var(--text-secondary)] transition-transform duration-150 ${open ? "rotate-180" : ""}`} />
      </button>
      {open && rect &&
        createPortal(
          <>
            <div className="fixed inset-0 z-[90]" onMouseDown={() => setOpen(false)} />
            <div
              className="fixed z-[91] flex flex-col rounded-xl border border-[var(--border-strong)] bg-[var(--bg-panel)]/95 backdrop-blur-md shadow-2xl overflow-hidden animate-pop-in"
              style={{ left: Math.max(8, Math.min(rect.left, window.innerWidth - rect.width - 8)), top: rect.top, width: rect.width, maxHeight: PANEL_HEIGHT }}
              onKeyDown={onKey}
            >
              <div className="flex items-center gap-2 px-2.5 h-9 border-b border-[var(--border-subtle)] bg-[var(--bg-sidebar)]/50">
                <Search size={13} className="text-[var(--text-secondary)] shrink-0" />
                <input
                  autoFocus
                  aria-label="Search actions"
                  aria-activedescendant={flat[cursor] ? `${pickerId}-opt-${cursor}` : undefined}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Search actions…"
                  className="flex-1 bg-transparent text-xs text-[var(--text-primary)] outline-none focus-ring-none placeholder:text-[var(--text-secondary)]"
                />
                {query && (
                  <button
                    type="button"
                    aria-label="Clear action search"
                    onClick={() => setQuery("")}
                    className="p-0.5 rounded text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] cursor-pointer transition-colors"
                  >
                    <X size={12} />
                  </button>
                )}
                <span className="text-[10px] font-mono text-[var(--text-muted)] tabular-nums">{flat.length}</span>
              </div>
              <div ref={listRef} className="overflow-y-auto py-1" role="listbox">
                {flat.length === 0 && (
                  <div className="px-3 py-6 text-center text-xs text-[var(--text-secondary)]">No action matches “{query}”.</div>
                )}
                {groups.map(([group, items]) => (
                  <div key={group}>
                    <div className="px-3 pt-2 pb-1 text-[10.5px] font-semibold text-[var(--text-muted)]">{group}</div>
                    {items.map((choice) => {
                      index += 1;
                      const i = index;
                      const active = i === cursor;
                      const selected = choice.id === value;
                      return (
                        <button
                          id={`${pickerId}-opt-${i}`}
                          key={choice.id}
                          type="button"
                          role="option"
                          aria-selected={selected}
                          data-index={i}
                          onMouseMove={() => setCursor(i)}
                          onClick={() => choose(choice)}
                          title={choice.description}
                          className={`mx-1.5 w-[calc(100%-0.75rem)] flex items-center gap-2 px-2.5 py-1.5 rounded-md text-left text-xs cursor-pointer transition-colors ${
                            active ? "bg-[var(--accent-soft)] text-[var(--text-primary)]" : ""
                          } ${selected ? "text-[var(--accent-text)] font-semibold" : "text-[var(--text-primary)]"}`}
                        >
                          {choice.iconName ? (
                            <DeckIcon name={choice.iconName} size={13} className="shrink-0 opacity-80" />
                          ) : (
                            <span className="w-[13px] shrink-0" />
                          )}
                          <span className="truncate flex-1">{nameOf(choice)}</span>
                          {selected && <Check size={12} className="shrink-0 text-[var(--accent-primary)]" />}
                        </button>
                      );
                    })}
                  </div>
                ))}
              </div>
            </div>
          </>,
          document.body
        )}
    </>
  );
};
