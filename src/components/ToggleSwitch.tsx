import React from "react";

interface ToggleSwitchProps {
  checked: boolean;
  onChange: () => void;
  ariaLabel: string;
  id?: string;
  size?: "sm" | "md";
  disabled?: boolean;
}

/** Accessible pill toggle switch with smooth thumb transition and tactile hover/active feedback. */
export const ToggleSwitch: React.FC<ToggleSwitchProps> = ({
  checked,
  onChange,
  ariaLabel,
  id,
  size = "md",
  disabled = false,
}) => {
  const isSmall = size === "sm";
  return (
    <button
      id={id}
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={onChange}
      className={`${
        isSmall ? "w-8 h-[18px]" : "w-9 h-5"
      } rounded-full p-0.5 shrink-0 cursor-pointer disabled:opacity-45 disabled:cursor-not-allowed border transition-all duration-150 active:scale-[0.95] disabled:active:scale-100 ${
        checked
          ? "bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] border-[var(--accent-hover)]/40 shadow-[0_0_10px_rgba(124,131,247,0.28)]"
          : "bg-[var(--border-strong)] hover:bg-[#424652] border-white/10"
      }`}
    >
      <span
        aria-hidden
        className={`block rounded-full bg-white shadow-xs transition-transform duration-150 ease-[cubic-bezier(0.22,1,0.36,1)] ${
          isSmall ? "w-3.5 h-3.5" : "w-4 h-4"
        } ${
          checked ? (isSmall ? "translate-x-[12px]" : "translate-x-[14px]") : "translate-x-0"
        }`}
      />
    </button>
  );
};
