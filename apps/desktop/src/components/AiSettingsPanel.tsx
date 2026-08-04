import { useState } from "react";

import { setAiSettings } from "../lib/atlas";
import { useAiStore } from "../store/aiStore";
import type { AiSettings } from "../types";

export default function AiSettingsPanel() {
  const settings = useAiStore((s) => s.settings);
  const setSettings = useAiStore((s) => s.setSettings);
  const [draft, setDraft] = useState<AiSettings>(settings);
  const [saved, setSaved] = useState(false);
  const [open, setOpen] = useState(false);

  function update(patch: Partial<AiSettings>) {
    setDraft((d) => ({ ...d, ...patch }));
    setSaved(false);
  }

  async function save() {
    await setAiSettings(draft);
    setSettings(draft);
    setSaved(true);
  }

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => {
          setDraft(settings);
          setOpen(true);
        }}
        className="text-xs text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)] mb-6"
      >
        Cloud AI settings {settings.cloud_enabled ? "(enabled)" : "(off)"}
      </button>
    );
  }

  return (
    <div className="rounded-lg border border-[color:var(--color-atlas-border)] p-4 mb-6">
      <div className="flex items-center justify-between mb-3">
        <p className="text-sm font-medium">Cloud AI</p>
        <button
          type="button"
          onClick={() => setOpen(false)}
          className="text-xs text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)]"
        >
          Close
        </button>
      </div>
      <p className="text-xs text-[color:var(--color-atlas-muted)] mb-4">
        Off by default. Nothing is ever sent off this machine unless you enable this and confirm
        each request individually. Only the text you type goes out, never file names or paths.
      </p>

      <label className="flex items-center gap-2 text-sm mb-3 cursor-pointer">
        <input
          type="checkbox"
          checked={draft.cloud_enabled}
          onChange={(e) => update({ cloud_enabled: e.target.checked })}
          className="accent-[color:var(--color-atlas-accent)]"
        />
        Enable cloud AI as an option
      </label>

      {draft.cloud_enabled && (
        <div className="space-y-2">
          <input
            type="text"
            placeholder="Base URL (e.g. https://api.openai.com/v1)"
            value={draft.cloud_base_url ?? ""}
            onChange={(e) => update({ cloud_base_url: e.target.value })}
            className="w-full rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-3 py-1.5 text-sm placeholder:text-[color:var(--color-atlas-muted)]"
          />
          <input
            type="text"
            placeholder="Model (e.g. gpt-4o-mini)"
            value={draft.cloud_model ?? ""}
            onChange={(e) => update({ cloud_model: e.target.value })}
            className="w-full rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-3 py-1.5 text-sm placeholder:text-[color:var(--color-atlas-muted)]"
          />
          <input
            type="password"
            placeholder="API key"
            value={draft.cloud_api_key ?? ""}
            onChange={(e) => update({ cloud_api_key: e.target.value })}
            className="w-full rounded-lg border border-[color:var(--color-atlas-border)] bg-transparent px-3 py-1.5 text-sm placeholder:text-[color:var(--color-atlas-muted)]"
          />
        </div>
      )}

      <div className="flex items-center gap-3 mt-3">
        <button
          type="button"
          onClick={() => void save()}
          className="rounded-lg bg-[color:var(--color-atlas-accent)] text-[#0b0d10] text-sm font-medium px-3 py-1.5"
        >
          Save
        </button>
        {saved && <span className="text-xs text-[color:var(--color-atlas-muted)]">Saved.</span>}
      </div>
    </div>
  );
}
