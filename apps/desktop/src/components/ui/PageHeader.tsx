import type { ReactNode } from "react";

interface Props {
  eyebrow: string;
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
}

/** The section-label + title + subtitle header every view opens with, plus an optional right-aligned action slot. */
export default function PageHeader({ eyebrow, title, subtitle, actions }: Props) {
  return (
    <div className="flex items-start justify-between gap-6 mb-6">
      <div>
        <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
          {eyebrow}
        </p>
        <h1 className="text-2xl font-semibold mb-1">{title}</h1>
        {subtitle && <p className="text-sm text-[color:var(--color-atlas-muted)]">{subtitle}</p>}
      </div>
      {actions && <div className="shrink-0">{actions}</div>}
    </div>
  );
}
