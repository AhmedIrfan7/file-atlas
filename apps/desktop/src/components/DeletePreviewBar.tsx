import { useState } from "react";

import { formatBytes } from "../lib/format";
import Button from "./ui/Button";

interface Props {
  pathCount: number;
  bytesToFree: number;
  paths: string[];
  onConfirm: () => void;
  busy: boolean;
}

export default function DeletePreviewBar({
  pathCount,
  bytesToFree,
  paths,
  onConfirm,
  busy,
}: Props) {
  const [confirming, setConfirming] = useState(false);
  const [prevPathCount, setPrevPathCount] = useState(pathCount);

  // The selection emptying out (a successful delete, a cancel, or manually
  // deselecting everything) always means the current review step is done;
  // without this, the bar stayed stuck on the final "Confirm delete" step
  // for the next unrelated selection instead of starting over at "Review".
  // Adjusting state during render rather than in an effect avoids an extra
  // committed frame of the stale step (react.dev/learn/you-might-not-need-an-effect).
  if (pathCount !== prevPathCount) {
    setPrevPathCount(pathCount);
    if (pathCount === 0) setConfirming(false);
  }

  if (pathCount === 0) return null;

  return (
    <div className="fixed bottom-0 inset-x-0 border-t border-[color:var(--color-atlas-border)] bg-[color:var(--color-atlas-surface)] px-6 py-4 shadow-[0_-8px_24px_-12px_rgba(0,0,0,0.5)]">
      <div className="max-w-3xl mx-auto">
        {confirming ? (
          <div>
            <p className="text-sm mb-2">
              Send <span className="font-semibold">{pathCount}</span> files (
              {formatBytes(bytesToFree)}) to the Recycle Bin?
            </p>
            <ul className="max-h-32 overflow-y-auto text-xs text-[color:var(--color-atlas-muted)] mb-3 space-y-0.5">
              {paths.map((p) => (
                <li key={p} className="truncate" title={p}>
                  {p}
                </li>
              ))}
            </ul>
            <div className="flex gap-2">
              <Button variant="danger" onClick={onConfirm} disabled={busy}>
                {busy ? "Deleting..." : "Confirm delete"}
              </Button>
              <Button variant="secondary" onClick={() => setConfirming(false)} disabled={busy}>
                Cancel
              </Button>
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-between">
            <p className="text-sm">
              <span className="font-semibold">{pathCount}</span> files selected,{" "}
              {formatBytes(bytesToFree)} to free
            </p>
            <Button variant="primary" onClick={() => setConfirming(true)}>
              Review &amp; delete
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
