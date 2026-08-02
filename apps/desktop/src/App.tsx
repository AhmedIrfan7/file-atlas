/**
 * File Atlas: M0 placeholder shell.
 *
 * This screen exists only to prove the Tauri + React + Tailwind pipeline works.
 * It is replaced in M2 by the real Home view.
 */
export default function App() {
  return (
    <main className="min-h-screen flex items-center justify-center px-6">
      <div className="max-w-xl w-full text-center">
        <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-4">
          File Atlas
        </p>
        <h1 className="text-4xl font-semibold leading-tight mb-4">
          The living map of everything on your computer.
        </h1>
        <p className="text-[color:var(--color-atlas-muted)] mb-8">
          M0 foundation. Nothing scans yet. The engine, the safety pipeline, and the map arrive in
          later milestones.
        </p>
        <div className="inline-flex items-center gap-2 rounded-full border border-[color:var(--color-atlas-border)] px-4 py-2 text-sm text-[color:var(--color-atlas-muted)]">
          <span className="h-2 w-2 rounded-full bg-[color:var(--color-atlas-accent)]" />
          Pre-alpha
        </div>
      </div>
    </main>
  );
}
