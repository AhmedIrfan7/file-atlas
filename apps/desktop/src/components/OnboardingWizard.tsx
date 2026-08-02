import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";

import { getDefaultRoots, startScan } from "../lib/atlas";
import { useScanStore } from "../store/scanStore";
import type { SuggestedRoot } from "../types";

export default function OnboardingWizard() {
  const [suggested, setSuggested] = useState<SuggestedRoot[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [customRoots, setCustomRoots] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState(false);
  const setScreen = useScanStore((s) => s.setScreen);
  const setError = useScanStore((s) => s.setError);

  useEffect(() => {
    getDefaultRoots()
      .then((roots) => {
        setSuggested(roots);
        setSelected(new Set(roots.map((r) => r.path)));
      })
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
  }, [setError]);

  function toggle(path: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }

  async function addCustomFolder() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === "string" && !customRoots.includes(picked)) {
      setCustomRoots((prev) => [...prev, picked]);
      setSelected((prev) => new Set(prev).add(picked));
    }
  }

  async function handleStart() {
    const roots = [...selected];
    if (roots.length === 0) return;
    setStarting(true);
    setError(null);
    try {
      await startScan(roots);
      setScreen("scanning");
    } catch (err) {
      setError(String(err));
      setStarting(false);
    }
  }

  const allRoots = [
    ...suggested,
    ...customRoots
      .filter((p) => !suggested.some((s) => s.path === p))
      .map((p) => ({ label: p, path: p })),
  ];

  return (
    <main className="min-h-screen flex items-center justify-center px-6">
      <div className="max-w-lg w-full">
        <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
          File Atlas
        </p>
        <h1 className="text-3xl font-semibold mb-2">What should Atlas look at first?</h1>
        <p className="text-[color:var(--color-atlas-muted)] mb-6">
          Pick the folders you want mapped. You can add more later.
        </p>

        {loading ? (
          <p className="text-sm text-[color:var(--color-atlas-muted)]">
            Looking for your folders...
          </p>
        ) : (
          <ul className="space-y-2 mb-4">
            {allRoots.map((root) => (
              <li key={root.path}>
                <label className="flex items-center gap-3 rounded-lg border border-[color:var(--color-atlas-border)] px-4 py-3 cursor-pointer hover:border-[color:var(--color-atlas-accent)] transition-colors">
                  <input
                    type="checkbox"
                    checked={selected.has(root.path)}
                    onChange={() => toggle(root.path)}
                    className="accent-[color:var(--color-atlas-accent)]"
                  />
                  <span className="flex-1 min-w-0">
                    <span className="block font-medium">{root.label}</span>
                    <span className="block text-xs text-[color:var(--color-atlas-muted)] truncate">
                      {root.path}
                    </span>
                  </span>
                </label>
              </li>
            ))}
          </ul>
        )}

        <button
          type="button"
          onClick={() => void addCustomFolder()}
          className="w-full mb-6 rounded-lg border border-dashed border-[color:var(--color-atlas-border)] px-4 py-3 text-sm text-[color:var(--color-atlas-muted)] hover:border-[color:var(--color-atlas-accent)] hover:text-[color:var(--color-atlas-fg)] transition-colors"
        >
          + Add a custom folder
        </button>

        <button
          type="button"
          onClick={() => void handleStart()}
          disabled={selected.size === 0 || starting}
          className="w-full rounded-lg bg-[color:var(--color-atlas-accent)] text-[#0b0d10] font-medium px-4 py-3 disabled:opacity-40 disabled:cursor-not-allowed transition-opacity"
        >
          {starting
            ? "Starting..."
            : `Scan ${selected.size} folder${selected.size === 1 ? "" : "s"}`}
        </button>
      </div>
    </main>
  );
}
