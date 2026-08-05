import type { ReactNode } from "react";

type Tone = "neutral" | "accent" | "success" | "warning" | "danger";

interface Props {
  tone?: Tone;
  children: ReactNode;
  className?: string;
}

const TONES: Record<Tone, string> = {
  neutral:
    "bg-[color:var(--color-atlas-surface-hover)] text-[color:var(--color-atlas-muted)] border-[color:var(--color-atlas-border)]",
  accent:
    "bg-[color:var(--color-atlas-accent)]/12 text-[color:var(--color-atlas-accent)] border-[color:var(--color-atlas-accent)]/25",
  success:
    "bg-[color:var(--color-atlas-success)]/12 text-[color:var(--color-atlas-success)] border-[color:var(--color-atlas-success)]/25",
  warning:
    "bg-[color:var(--color-atlas-warning)]/12 text-[color:var(--color-atlas-warning)] border-[color:var(--color-atlas-warning)]/25",
  danger:
    "bg-[color:var(--color-atlas-danger)]/12 text-[color:var(--color-atlas-danger)] border-[color:var(--color-atlas-danger)]/25",
};

/** A small filled-tint pill for confidence levels, categories, and status labels. */
export default function Badge({ tone = "neutral", children, className = "" }: Props) {
  return (
    <span
      className={`inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium ${TONES[tone]} ${className}`}
    >
      {children}
    </span>
  );
}
