import { cancelScan } from "../lib/atlas";
import { formatBytes } from "../lib/format";
import { useScanStore } from "../store/scanStore";
import Button from "./ui/Button";

export default function ScanningView() {
  const progress = useScanStore((s) => s.progress);

  return (
    <main className="min-h-screen flex items-center justify-center px-6">
      <div className="max-w-md w-full text-center">
        <div
          className="mx-auto mb-6 h-10 w-10 rounded-full border-2 border-[color:var(--color-atlas-border)] border-t-[color:var(--color-atlas-accent)] animate-spin"
          aria-hidden="true"
        />
        <h1 className="text-2xl font-semibold mb-2">Mapping your files</h1>
        <p className="text-[color:var(--color-atlas-muted)] mb-1 truncate">
          {progress.currentRoot ?? "Starting scan..."}
        </p>
        <p className="text-sm text-[color:var(--color-atlas-muted)] mb-8">
          {progress.filesSeen.toLocaleString()} files, {formatBytes(progress.bytesSeen)} seen so far
        </p>
        <Button onClick={() => void cancelScan()}>Cancel</Button>
      </div>
    </main>
  );
}
