interface Props {
  value: string;
  onChange: (value: string) => void;
  onSave: () => void;
  canSave: boolean;
}

export default function SearchBar({ value, onChange, onSave, canSave }: Props) {
  return (
    <div className="flex items-center gap-3">
      <input
        type="text"
        autoFocus
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="resume  type:pdf  size>10mb  age<1y  in:downloads"
        className="flex-1 rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-4 py-3 text-sm placeholder:text-[color:var(--color-atlas-muted)] focus:outline-none focus:border-[color:var(--color-atlas-accent)]"
      />
      <button
        type="button"
        onClick={onSave}
        disabled={!canSave}
        className="shrink-0 rounded-lg border border-[color:var(--color-atlas-border)] px-4 py-3 text-sm text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)] hover:border-[color:var(--color-atlas-accent)] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
      >
        Save
      </button>
    </div>
  );
}
