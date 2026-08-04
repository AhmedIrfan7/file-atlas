import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useEffect, useState } from "react";

// The build produces this from package.json at build time via Vite's define
// config in vite.config.ts, so the version shown here never drifts from
// what actually got packaged.
declare const __APP_VERSION__: string;

type Status = "checking" | "up-to-date" | "available" | "installing" | "error";

export default function UpdateChecker() {
  const [status, setStatus] = useState<Status>("checking");
  const [update, setUpdate] = useState<Update | null>(null);

  useEffect(() => {
    check()
      .then((result) => {
        if (result) {
          setUpdate(result);
          setStatus("available");
        } else {
          setStatus("up-to-date");
        }
      })
      .catch(() => setStatus("error"));
  }, []);

  async function installAndRestart() {
    if (!update) return;
    setStatus("installing");
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch {
      setStatus("error");
    }
  }

  return (
    <div className="flex items-center gap-3 text-xs text-[color:var(--color-atlas-muted)]">
      <span>v{__APP_VERSION__}</span>
      {status === "available" && update && (
        <button
          type="button"
          onClick={() => void installAndRestart()}
          className="rounded-md border border-[color:var(--color-atlas-accent)] px-2 py-1 text-[color:var(--color-atlas-accent)] hover:bg-[color:var(--color-atlas-accent)]/10 transition-colors"
        >
          Update to v{update.version} available &middot; Install &amp; restart
        </button>
      )}
      {status === "installing" && <span>Installing update...</span>}
      {status === "error" && <span title="Could not check for updates">Update check failed</span>}
    </div>
  );
}
