/**
 * The shared UI kit: every screen builds its controls from these, so sizes, colours, focus
 * and disabled states match across the app. Colours come from the tokens in src/index.css.
 */
import React, { useEffect, useId, useRef, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { Check, ChevronDown, ChevronRight } from "lucide-react";

/** Joins class names, skipping empty ones. */
export function cn(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}

// ---------------------------------------------------------------- buttons

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "subtle";
export type ButtonSize = "sm" | "md" | "lg";

const BUTTON_VARIANTS: Record<ButtonVariant, string> = {
  primary: "bg-accent text-white hover:bg-accent-hover border border-accent-hover/40 shadow-sm shadow-black/30",
  secondary: "bg-studio-800 text-studio-100 hover:bg-studio-700 border border-studio-700 hover:border-studio-600",
  ghost: "bg-transparent text-studio-300 hover:text-studio-100 hover:bg-studio-800 border border-transparent",
  subtle: "bg-accent/15 text-accent-fg hover:bg-accent/25 border border-accent/40",
  danger: "bg-danger/15 text-danger-fg hover:bg-danger/25 border border-danger/40",
};

const BUTTON_SIZES: Record<ButtonSize, string> = {
  sm: "h-control-sm px-2.5 gap-1.5 text-meta",
  md: "h-control px-3 gap-1.5 text-label",
  lg: "h-control-lg px-4 gap-2 text-body",
};

const ICON_SIZES: Record<ButtonSize, string> = {
  sm: "w-3.5 h-3.5",
  md: "w-4 h-4",
  lg: "w-4 h-4",
};

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: LucideIcon;
  /** Shown after the label (a chevron, a count). */
  trailing?: React.ReactNode;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = "secondary", size = "md", icon: Icon, trailing, className, children, type = "button", ...rest }, ref) => (
    <button
      ref={ref}
      type={type}
      className={cn(
        "inline-flex items-center justify-center shrink-0 rounded-control font-medium whitespace-nowrap transition-colors disabled:opacity-45 disabled:pointer-events-none",
        BUTTON_VARIANTS[variant],
        BUTTON_SIZES[size],
        className,
      )}
      {...rest}
    >
      {Icon && <Icon className={cn(ICON_SIZES[size], "shrink-0")} aria-hidden />}
      {children}
      {trailing}
    </button>
  ),
);
Button.displayName = "Button";

export interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  /** Read by screen readers and shown as the tooltip. */
  label: string;
  size?: ButtonSize;
  variant?: "ghost" | "secondary" | "primary" | "danger";
  /** A toggle that is on. */
  active?: boolean;
}

const ICON_BUTTON_SIZES: Record<ButtonSize, string> = {
  sm: "w-7 h-7",
  md: "w-8 h-8",
  lg: "w-9 h-9",
};

export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ icon: Icon, label, size = "md", variant = "ghost", active, className, title, type = "button", ...rest }, ref) => (
    <button
      ref={ref}
      type={type}
      aria-label={label}
      title={title ?? label}
      aria-pressed={active}
      className={cn(
        "inline-flex items-center justify-center shrink-0 rounded-control transition-colors disabled:opacity-40 disabled:pointer-events-none",
        ICON_BUTTON_SIZES[size],
        active
          ? "bg-accent/20 text-accent-fg border border-accent/40"
          : variant === "ghost"
            ? "text-studio-400 hover:text-studio-100 hover:bg-studio-800 border border-transparent"
            : BUTTON_VARIANTS[variant],
        className,
      )}
      {...rest}
    >
      <Icon className={ICON_SIZES[size]} aria-hidden />
    </button>
  ),
);
IconButton.displayName = "IconButton";

// ---------------------------------------------------------------- choices

/** An on/off switch. */
export const Switch: React.FC<{
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: React.ReactNode;
  disabled?: boolean;
  title?: string;
  className?: string;
}> = ({ checked, onChange, label, disabled, title, className }) => (
  <label
    className={cn(
      "inline-flex items-center gap-2 text-label text-studio-200 select-none",
      disabled ? "opacity-45 cursor-not-allowed" : "cursor-pointer",
      className,
    )}
    title={title}
  >
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors",
        checked ? "bg-accent border-accent-hover" : "bg-studio-800 border-studio-600",
      )}
    >
      <span
        className={cn(
          "inline-block h-3.5 w-3.5 rounded-full bg-white shadow transition-transform",
          checked ? "translate-x-[18px]" : "translate-x-[3px]",
        )}
      />
    </button>
    {label}
  </label>
);

export interface SegmentOption<T extends string | number> {
  value: T;
  label: React.ReactNode;
  title?: string;
  disabled?: boolean;
}

/** A row of mutually exclusive choices. */
export function Segmented<T extends string | number>({
  options,
  value,
  onChange,
  size = "md",
  label,
  className,
}: {
  options: SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  size?: "sm" | "md";
  /** Accessible name of the group. */
  label: string;
  className?: string;
}) {
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className={cn("inline-flex rounded-control border border-studio-700 bg-studio-900 p-0.5 gap-0.5", className)}
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={String(option.value)}
            type="button"
            role="radio"
            aria-checked={selected}
            title={option.title}
            disabled={option.disabled}
            onClick={() => onChange(option.value)}
            className={cn(
              "flex-1 inline-flex items-center justify-center gap-1.5 rounded-[5px] px-2.5 font-medium whitespace-nowrap transition-colors disabled:opacity-40",
              size === "sm" ? "h-6 text-meta" : "h-7 text-label",
              selected
                ? "bg-accent/20 text-accent-fg shadow-[inset_0_0_0_1px_rgb(var(--accent-hover)/0.55)]"
                : "text-studio-400 hover:text-studio-100 hover:bg-studio-800",
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

export interface TabItem<T extends string> {
  value: T;
  label: React.ReactNode;
  icon?: LucideIcon;
  title?: string;
}

/** Underlined tabs: workspaces, panel pages. */
export function Tabs<T extends string>({
  items,
  value,
  onChange,
  label,
  className,
  size = "md",
}: {
  items: TabItem<T>[];
  value: T | null;
  onChange: (value: T) => void;
  label: string;
  className?: string;
  size?: "sm" | "md";
}) {
  return (
    <div role="tablist" aria-label={label} className={cn("flex items-stretch gap-1", className)}>
      {items.map(({ value: item, label: text, icon: Icon, title }) => {
        const selected = item === value;
        return (
          <button
            key={item}
            type="button"
            role="tab"
            aria-selected={selected}
            title={title}
            onClick={() => onChange(item)}
            className={cn(
              "relative inline-flex items-center gap-1.5 px-3 font-medium transition-colors",
              size === "sm" ? "text-label" : "text-body",
              selected ? "text-studio-100" : "text-studio-400 hover:text-studio-100",
            )}
          >
            {Icon && <Icon className="w-4 h-4" aria-hidden />}
            {text}
            <span
              aria-hidden
              className={cn(
                "absolute left-2 right-2 -bottom-px h-0.5 rounded-full transition-colors",
                selected ? "bg-accent-hover" : "bg-transparent",
              )}
            />
          </button>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------- menu

export type MenuEntry =
  | {
      kind?: "item";
      label: string;
      icon?: LucideIcon;
      /** Shown on the right: a shortcut or a hint. */
      hint?: string;
      onSelect: () => void;
      disabled?: boolean;
      /** Why it is disabled, shown under the label. */
      disabledReason?: string;
      checked?: boolean;
      danger?: boolean;
    }
  | { kind: "separator" }
  | { kind: "heading"; label: string }
  | { kind: "submenu"; label: string; icon?: LucideIcon; entries: MenuEntry[]; disabled?: boolean };

/** A dropdown menu on a button: Escape closes it, arrows move, Enter chooses. */
export const Menu: React.FC<{
  trigger: (props: { open: boolean; toggle: () => void; ref: React.Ref<HTMLButtonElement> }) => React.ReactNode;
  entries: MenuEntry[];
  align?: "left" | "right";
  width?: number;
  label: string;
}> = ({ trigger, entries, align = "left", width = 260, label }) => {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", onPointerDown);
    // Focus the first item, so the keyboard can drive the menu at once.
    const focusFirst = window.setTimeout(() =>
      listRef.current?.querySelector<HTMLButtonElement>("[role=menuitem]:not(:disabled)")?.focus(),
    );
    return () => {
      window.clearTimeout(focusFirst);
      window.removeEventListener("pointerdown", onPointerDown);
    };
  }, [open]);

  const close = (refocus = true) => {
    setOpen(false);
    if (refocus) triggerRef.current?.focus();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      close();
      return;
    }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const items = [...(listRef.current?.querySelectorAll<HTMLButtonElement>("[role=menuitem]:not(:disabled)") ?? [])];
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = items[(at + (event.key === "ArrowDown" ? 1 : items.length - 1)) % items.length];
    next?.focus();
  };

  return (
    <div ref={rootRef} className="relative" onKeyDown={onKeyDown}>
      {trigger({ open, toggle: () => setOpen((value) => !value), ref: triggerRef })}
      {open && (
        <div
          ref={listRef}
          role="menu"
          aria-label={label}
          style={{ width }}
          className={cn(
            "absolute top-full mt-1.5 z-[1000] rounded-panel border border-studio-700 bg-studio-850 shadow-popover p-1",
            align === "right" ? "right-0" : "left-0",
          )}
        >
          <MenuEntries entries={entries} onDone={() => close(false)} />
        </div>
      )}
    </div>
  );
};

const MenuEntries: React.FC<{ entries: MenuEntry[]; onDone: () => void }> = ({ entries, onDone }) => (
  <>
    {entries.map((entry, index) => {
      if (entry.kind === "separator") return <div key={index} role="separator" className="my-1 h-px bg-studio-700/70" />;
      if (entry.kind === "heading")
        return (
          <div key={index} className="px-2.5 pt-2 pb-1 text-meta font-semibold uppercase tracking-wide text-studio-500">
            {entry.label}
          </div>
        );
      if (entry.kind === "submenu") return <SubMenu key={index} entry={entry} onDone={onDone} />;
      const Icon = entry.icon;
      return (
        <button
          key={index}
          type="button"
          role={entry.checked === undefined ? "menuitem" : "menuitemcheckbox"}
          aria-checked={entry.checked}
          disabled={entry.disabled}
          onClick={() => {
            onDone();
            entry.onSelect();
          }}
          className={cn(
            "w-full flex items-start gap-2.5 rounded-control px-2.5 py-1.5 text-left text-label transition-colors focus-visible:shadow-none focus:bg-studio-700/70",
            entry.danger ? "text-danger-fg hover:bg-danger/15" : "text-studio-100 hover:bg-studio-700/70",
            "disabled:opacity-100 disabled:text-studio-500",
          )}
        >
          <span className="w-4 h-[18px] flex items-center justify-center shrink-0">
            {entry.checked ? <Check className="w-4 h-4 text-accent-fg" /> : Icon ? <Icon className="w-4 h-4 text-studio-400" /> : null}
          </span>
          <span className="flex-1 min-w-0">
            <span className="block truncate">{entry.label}</span>
            {entry.disabled && entry.disabledReason && (
              <span className="block text-meta text-studio-500">{entry.disabledReason}</span>
            )}
          </span>
          {entry.hint && <span className="text-meta text-studio-500 whitespace-nowrap pt-px">{entry.hint}</span>}
        </button>
      );
    })}
  </>
);

const SubMenu: React.FC<{
  entry: Extract<MenuEntry, { kind: "submenu" }>;
  onDone: () => void;
}> = ({ entry, onDone }) => {
  const [open, setOpen] = useState(false);
  const Icon = entry.icon;
  return (
    <div className="relative" onMouseEnter={() => setOpen(true)} onMouseLeave={() => setOpen(false)}>
      <button
        type="button"
        role="menuitem"
        aria-haspopup="menu"
        aria-expanded={open}
        disabled={entry.disabled}
        onClick={() => setOpen((value) => !value)}
        onKeyDown={(event) => {
          if (event.key === "ArrowRight") {
            event.preventDefault();
            setOpen(true);
          }
        }}
        className="w-full flex items-center gap-2.5 rounded-control px-2.5 py-1.5 text-left text-label text-studio-100 hover:bg-studio-700/70 focus:bg-studio-700/70 focus-visible:shadow-none disabled:text-studio-500"
      >
        <span className="w-4 h-4 flex items-center justify-center shrink-0">
          {Icon && <Icon className="w-4 h-4 text-studio-400" />}
        </span>
        <span className="flex-1 truncate">{entry.label}</span>
        <ChevronRight className="w-3.5 h-3.5 text-studio-500" />
      </button>
      {open && (
        <div
          role="menu"
          aria-label={entry.label}
          className="absolute left-full top-0 -mt-1 ml-1 w-72 rounded-panel border border-studio-700 bg-studio-850 shadow-popover p-1"
          onKeyDown={(event) => {
            if (event.key === "ArrowLeft") {
              event.stopPropagation();
              setOpen(false);
            }
          }}
        >
          <MenuEntries entries={entry.entries} onDone={onDone} />
        </div>
      )}
    </div>
  );
};

/** The usual trigger for a menu: a ghost button with a chevron. */
export const MenuButton = React.forwardRef<
  HTMLButtonElement,
  { open: boolean; onClick: () => void; children: React.ReactNode; icon?: LucideIcon; className?: string; title?: string }
>(({ open, onClick, children, icon, className, title }, ref) => (
  <Button
    ref={ref}
    variant="ghost"
    icon={icon}
    aria-haspopup="menu"
    aria-expanded={open}
    onClick={onClick}
    title={title}
    className={cn(open && "bg-studio-800 text-studio-100", className)}
    trailing={<ChevronDown className={cn("w-3.5 h-3.5 text-studio-500 transition-transform", open && "rotate-180")} />}
  >
    {children}
  </Button>
));
MenuButton.displayName = "MenuButton";

// ---------------------------------------------------------------- small pieces

export type Tone = "neutral" | "accent" | "video" | "audio" | "caption" | "zoom" | "suggest" | "danger" | "success";

const TONES: Record<Tone, string> = {
  neutral: "bg-studio-800 text-studio-300 border-studio-700",
  accent: "bg-accent/15 text-accent-fg border-accent/40",
  video: "bg-video/15 text-video-fg border-video/40",
  audio: "bg-audio/15 text-audio-fg border-audio/40",
  caption: "bg-caption-fill text-caption border-caption/40",
  zoom: "bg-zoom/15 text-zoom-fg border-zoom/40",
  suggest: "bg-suggest/15 text-suggest-fg border-suggest/40",
  danger: "bg-danger/15 text-danger-fg border-danger/40",
  success: "bg-success/15 text-success-fg border-success/40",
};

/** A small status label. */
export const Badge: React.FC<{ tone?: Tone; children: React.ReactNode; className?: string; title?: string }> = ({
  tone = "neutral",
  children,
  className,
  title,
}) => (
  <span
    title={title}
    className={cn(
      "inline-flex items-center gap-1 h-5 px-1.5 rounded border text-meta font-medium whitespace-nowrap",
      TONES[tone],
      className,
    )}
  >
    {children}
  </span>
);

/** A key on the keyboard, as in a shortcut hint. */
export const Kbd: React.FC<{ children: React.ReactNode; className?: string }> = ({ children, className }) => (
  <kbd
    className={cn(
      "inline-flex items-center h-5 min-w-[20px] justify-center px-1.5 rounded border border-studio-700 bg-studio-850 font-sans text-meta text-studio-300",
      className,
    )}
  >
    {children}
  </kbd>
);

/** The title row of a panel: a name, an optional icon, actions on the right. */
export const PanelHeader: React.FC<{
  title: React.ReactNode;
  icon?: LucideIcon;
  /** A quieter detail after the title. */
  detail?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
}> = ({ title, icon: Icon, detail, actions, className }) => (
  <div className={cn("flex items-center gap-2 min-h-[40px] px-3 border-b border-studio-800", className)}>
    {Icon && <Icon className="w-4 h-4 text-studio-400 shrink-0" aria-hidden />}
    <div className="min-w-0 flex-1 flex items-baseline gap-2">
      <h2 className="text-label font-semibold text-studio-100 truncate">{title}</h2>
      {detail && <span className="text-meta text-studio-500 truncate">{detail}</span>}
    </div>
    {actions && <div className="flex items-center gap-1 shrink-0">{actions}</div>}
  </div>
);

/** A labelled row in a settings list: label left, control right. */
export const Field: React.FC<{
  label: React.ReactNode;
  hint?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}> = ({ label, hint, children, className }) => {
  const id = useId();
  return (
    <div className={cn("grid grid-cols-[7rem_minmax(0,1fr)] items-center gap-x-3 gap-y-1", className)}>
      <span id={id} className="text-label text-studio-400">
        {label}
      </span>
      <div aria-labelledby={id} className="min-w-0">
        {children}
      </div>
      {hint && <p className="col-start-2 text-meta text-studio-500">{hint}</p>}
    </div>
  );
};

/** A message in a strip: errors, notices. */
export const Notice: React.FC<{
  tone?: "danger" | "accent" | "suggest" | "neutral";
  children: React.ReactNode;
  onDismiss?: () => void;
  action?: React.ReactNode;
  className?: string;
}> = ({ tone = "neutral", children, onDismiss, action, className }) => (
  <div
    role={tone === "danger" ? "alert" : "status"}
    className={cn(
      "flex items-center gap-3 px-4 py-2 text-label border-b",
      tone === "danger" && "bg-danger/10 border-danger/30 text-danger-fg",
      tone === "accent" && "bg-accent/10 border-accent/30 text-accent-fg",
      tone === "suggest" && "bg-suggest/10 border-suggest/30 text-suggest-fg",
      tone === "neutral" && "bg-studio-900 border-studio-800 text-studio-300",
      className,
    )}
  >
    <div className="flex-1 min-w-0">{children}</div>
    {action}
    {onDismiss && (
      <button
        type="button"
        onClick={onDismiss}
        aria-label="Dismiss"
        className="h-7 w-7 inline-flex items-center justify-center rounded-control opacity-70 hover:opacity-100 hover:bg-white/5"
      >
        ×
      </button>
    )}
  </div>
);
