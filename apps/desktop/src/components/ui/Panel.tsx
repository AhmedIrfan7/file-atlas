import type { HTMLAttributes } from "react";

interface Props extends HTMLAttributes<HTMLDivElement> {
  interactive?: boolean;
}

/**
 * The raised-surface container every card/panel in the app should use
 * instead of a bare border on the page background. A real elevation step
 * (surface color, not just an outline) is what makes content read as
 * grouped rather than merely boxed.
 */
export default function Panel({ interactive = false, className = "", ...rest }: Props) {
  return (
    <div
      className={`rounded-lg border border-[color:var(--color-atlas-border)] bg-[color:var(--color-atlas-surface)] ${
        interactive ? "transition-colors hover:bg-[color:var(--color-atlas-surface-hover)]" : ""
      } ${className}`}
      {...rest}
    />
  );
}
