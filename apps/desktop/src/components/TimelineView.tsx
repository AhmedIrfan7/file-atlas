import { useEffect } from "react";

import { getLifeTimeline, getProjectBursts, getScreenshotBursts } from "../lib/atlas";
import { TIMELINE_VIEWS, useTimelineStore } from "../store/timelineStore";
import BurstCard from "./BurstCard";
import TimelineChart from "./TimelineChart";

export default function TimelineView() {
  const view = useTimelineStore((s) => s.view);
  const buckets = useTimelineStore((s) => s.buckets);
  const screenshotBursts = useTimelineStore((s) => s.screenshotBursts);
  const projectBursts = useTimelineStore((s) => s.projectBursts);
  const loading = useTimelineStore((s) => s.loading);
  const error = useTimelineStore((s) => s.error);
  const setView = useTimelineStore((s) => s.setView);
  const setBuckets = useTimelineStore((s) => s.setBuckets);
  const setBursts = useTimelineStore((s) => s.setBursts);
  const setLoading = useTimelineStore((s) => s.setLoading);
  const setError = useTimelineStore((s) => s.setError);

  useEffect(() => {
    setLoading(true);
    getLifeTimeline(view.granularity, view.sinceDays)
      .then((resp) => setBuckets(resp.buckets))
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view]);

  useEffect(() => {
    Promise.all([getScreenshotBursts(), getProjectBursts()])
      .then(([screenshots, projects]) => setBursts(screenshots, projects))
      .catch((err: unknown) => setError(String(err)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <main className="min-h-screen px-6 py-10 max-w-4xl mx-auto">
      <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
        Timeline
      </p>
      <h1 className="text-2xl font-semibold mb-1">A chronological view of your digital life</h1>
      <p className="text-sm text-[color:var(--color-atlas-muted)] mb-6">
        {loading ? "Loading..." : `${buckets.length.toLocaleString()} periods with new files`}
      </p>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <div className="flex items-center gap-1 rounded-lg border border-[color:var(--color-atlas-border)] p-1 mb-4 w-fit">
        {TIMELINE_VIEWS.map((v) => (
          <button
            key={v.label}
            type="button"
            onClick={() => setView(v)}
            className={`rounded-md px-2.5 py-1 text-xs transition-colors ${
              view.label === v.label
                ? "bg-white/10 text-[color:var(--color-atlas-fg)]"
                : "text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)]"
            }`}
          >
            {v.label}
          </button>
        ))}
      </div>

      <TimelineChart buckets={buckets} granularity={view.granularity} />

      {screenshotBursts.length > 0 && (
        <section className="mt-8">
          <h2 className="text-sm font-semibold mb-3">Screenshot bursts</h2>
          <div className="grid gap-3 sm:grid-cols-2">
            {screenshotBursts.map((burst) => (
              <BurstCard key={`${burst.kind}-${burst.period_start}`} burst={burst} />
            ))}
          </div>
        </section>
      )}

      {projectBursts.length > 0 && (
        <section className="mt-8">
          <h2 className="text-sm font-semibold mb-3">Project bursts</h2>
          <div className="grid gap-3 sm:grid-cols-2">
            {projectBursts.map((burst) => (
              <BurstCard
                key={`${burst.kind}-${burst.folder}-${burst.period_start}`}
                burst={burst}
              />
            ))}
          </div>
        </section>
      )}
    </main>
  );
}
