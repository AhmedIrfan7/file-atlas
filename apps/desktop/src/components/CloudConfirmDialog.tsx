import Button from "./ui/Button";

interface Props {
  query: string;
  cloudModel: string | null;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function CloudConfirmDialog({ query, cloudModel, onConfirm, onCancel }: Props) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 px-6">
      <div className="w-full max-w-md rounded-lg border border-[color:var(--color-atlas-border)] bg-[color:var(--color-atlas-surface)] p-5 shadow-2xl">
        <p className="text-sm font-medium mb-2">Send this to the cloud?</p>
        <p className="text-sm text-[color:var(--color-atlas-muted)] mb-4">
          This exact text will be sent to {cloudModel ?? "your configured cloud model"}. Nothing
          else, no file names or paths, leaves this machine.
        </p>
        <div className="rounded-lg border border-[color:var(--color-atlas-border)] bg-[color:var(--color-atlas-bg)] px-3 py-2 text-sm mb-4 break-words">
          &quot;{query}&quot;
        </div>
        <div className="flex justify-end gap-3">
          <Button variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="primary" onClick={onConfirm}>
            Send
          </Button>
        </div>
      </div>
    </div>
  );
}
