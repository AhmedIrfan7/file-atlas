import type { InputHTMLAttributes } from "react";

type Props = InputHTMLAttributes<HTMLInputElement>;

export default function Input({ className = "", ...rest }: Props) {
  return (
    <input
      className={`rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-3 py-2 text-sm text-[color:var(--color-atlas-fg)] placeholder:text-[color:var(--color-atlas-muted)] transition-colors focus:outline-none focus:border-[color:var(--color-atlas-accent)] ${className}`}
      {...rest}
    />
  );
}
