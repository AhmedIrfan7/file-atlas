interface Option {
  key: string;
  label: string;
}

interface Props {
  options: readonly Option[];
  activeKey: string;
  onSelect: (key: string) => void;
  className?: string;
}

/** The pill-group tab control used for nav tabs and view/window switchers alike. */
export default function SegmentedControl({ options, activeKey, onSelect, className = "" }: Props) {
  return (
    <div
      className={`flex items-center gap-1 rounded-lg border border-[color:var(--color-atlas-border)] p-1 ${className}`}
    >
      {options.map((option) => (
        <button
          key={option.key}
          type="button"
          onClick={() => onSelect(option.key)}
          className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
            option.key === activeKey
              ? "bg-[color:var(--color-atlas-accent)]/12 text-[color:var(--color-atlas-accent)]"
              : "text-[color:var(--color-atlas-muted)] hover:bg-[color:var(--color-atlas-surface-hover)] hover:text-[color:var(--color-atlas-fg)]"
          }`}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
