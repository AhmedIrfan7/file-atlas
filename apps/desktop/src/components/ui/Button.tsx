import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  children: ReactNode;
}

const BASE =
  "inline-flex items-center justify-center gap-1.5 rounded-lg font-medium transition-colors " +
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-atlas-accent)] " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-[color:var(--color-atlas-bg)] " +
  "disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none";

const VARIANTS: Record<Variant, string> = {
  primary:
    "bg-[color:var(--color-atlas-accent)] text-[color:var(--color-atlas-accent-ink)] " +
    "hover:bg-[color:var(--color-atlas-accent-hover)]",
  secondary:
    "border border-[color:var(--color-atlas-border)] text-[color:var(--color-atlas-fg)] " +
    "hover:border-[color:var(--color-atlas-accent)] hover:bg-[color:var(--color-atlas-surface-hover)]",
  ghost:
    "text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)] " +
    "hover:bg-[color:var(--color-atlas-surface-hover)]",
  danger:
    "bg-[color:var(--color-atlas-danger)] text-[color:var(--color-atlas-danger-ink)] hover:opacity-90",
};

const SIZES: Record<Size, string> = {
  sm: "px-2.5 py-1 text-xs",
  md: "px-4 py-2 text-sm",
};

export default function Button({
  variant = "secondary",
  size = "md",
  className = "",
  children,
  ...rest
}: Props) {
  return (
    <button
      type="button"
      className={`${BASE} ${VARIANTS[variant]} ${SIZES[size]} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
