interface Props {
  query: string;
  cloudModel: string | null;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function CloudConfirmDialog({ query, cloudModel, onConfirm, onCancel }: Props) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 px-6">
      <div className="w-full max-w-md rounded-lg border border-[color:var(--color-atlas-border)] bg-[#0b0d10] p-5">
        <p className="text-sm font-medium mb-2">Send this to the cloud?</p>
        <p className="text-sm text-[color:var(--color-atlas-muted)] mb-4">
          This exact text will be sent to {cloudModel ?? "your configured cloud model"}. Nothing
          else, no file names or paths, leaves this machine.
        </p>
        <div className="rounded-lg border border-[color:var(--color-atlas-border)] px-3 py-2 text-sm mb-4 break-words">
          &quot;{query}&quot;
        </div>
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            className="rounded-lg border border-[color:var(--color-atlas-border)] px-3 py-1.5 text-sm text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)]"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className="rounded-lg bg-[color:var(--color-atlas-accent)] text-[#0b0d10] text-sm font-medium px-3 py-1.5"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}
