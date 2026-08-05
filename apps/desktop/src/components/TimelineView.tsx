import { useEffect } from "react";

import { getLifeTimeline, getProjectBursts, getScreenshotBursts } from "../lib/atlas";
import { TIMELINE_VIEWS, useTimelineStore } from "../store/timelineStore";
import BurstCard from "./BurstCard";
import TimelineChart from "./TimelineChart";
import PageHeader from "./ui/PageHeader";
import SegmentedControl from "./ui/SegmentedControl";

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
    // Bursts used to always query all-time regardless of the view selector
    // above: switching to "This week" changed the histogram but silently
    // left the burst cards showing old, unrelated data from months back.
    Promise.all([getScreenshotBursts(view.sinceDays), getProjectBursts(view.sinceDays)])
      .then(([screenshots, projects]) => setBursts(screenshots, projects))
      .catch((err: unknown) => setError(String(err)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view]);

  return (
    <main className="min-h-screen px-6 py-10 max-w-4xl mx-auto">
      <PageHeader
        eyebrow="Timeline"
        title="A chronological view of your digital life"
        subtitle={
          loading ? "Loading..." : `${buckets.length.toLocaleString()} periods with new files`
        }
      />

      {error && <p className="text-sm text-[color:var(--color-atlas-danger)] mb-4">{error}</p>}

      <SegmentedControl
        options={TIMELINE_VIEWS.map((v) => ({ key: v.label, label: v.label }))}
        activeKey={view.label}
        onSelect={(key) => {
          const next = TIMELINE_VIEWS.find((v) => v.label === key);
          if (next) setView(next);
        }}
        className="mb-4 w-fit"
      />

      <TimelineChart buckets={buckets} granularity={view.granularity} />

      {screenshotBursts.length > 0 && (
        <section className="mt-8">
          <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
            Screenshot bursts
          </h2>
          <div className="grid gap-3 sm:grid-cols-2">
            {screenshotBursts.map((burst) => (
              <BurstCard key={`${burst.kind}-${burst.period_start}`} burst={burst} />
            ))}
          </div>
        </section>
      )}

      {projectBursts.length > 0 && (
        <section className="mt-8">
          <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
            Project bursts
          </h2>
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
