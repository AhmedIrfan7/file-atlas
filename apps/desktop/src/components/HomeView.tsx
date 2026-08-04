import { useEffect, useRef, useState } from "react";

import { getHomeSummary, getStaleBucket, getTopLargest, getTopOldest } from "../lib/atlas";
import { formatBytes } from "../lib/format";
import { useScanStore } from "../store/scanStore";
import type { FileSummary, HomeSummary, StaleBucket } from "../types";
import CategoryBreakdown from "./CategoryBreakdown";
import StaleBucketCard from "./StaleBucketCard";
import TopFilesList from "./TopFilesList";

const TOP_N = 10;
const STALE_MIN_AGE_DAYS = 365;

export default function HomeView() {
  const [summary, setSummary] = useState<HomeSummary | null>(null);
  const [largest, setLargest] = useState<FileSummary[]>([]);
  const [oldest, setOldest] = useState<FileSummary[]>([]);
  const [stale, setStale] = useState<StaleBucket | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const setScreen = useScanStore((s) => s.setScreen);
  const setError = useScanStore((s) => s.setError);
  const loadGeneration = useRef(0);

  // Does not reset `loading`/`loadError` itself: the initial render already
  // starts in that state, and the retry button resets it explicitly before
  // calling this, since doing it here would mean a plain useState setter
  // gets called synchronously from inside the mount effect below (flagged
  // by react-hooks/set-state-in-effect; the same pattern elsewhere in this
  // codebase only avoids it because those views' loading/error state lives
  // in a Zustand store instead of a local useState).
  function load() {
    const generation = ++loadGeneration.current;
    Promise.all([
      getHomeSummary(),
      getTopLargest(TOP_N),
      getTopOldest(TOP_N),
      getStaleBucket(STALE_MIN_AGE_DAYS, 5),
    ])
      .then(([s, l, o, st]) => {
        if (generation !== loadGeneration.current) return;
        setSummary(s);
        setLargest(l);
        setOldest(o);
        setStale(st);
      })
      .catch((err: unknown) => {
        if (generation !== loadGeneration.current) return;
        setLoadError(String(err));
        setError(String(err));
      })
      .finally(() => {
        if (generation === loadGeneration.current) setLoading(false);
      });
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (loadError) {
    return (
      <main className="min-h-screen flex flex-col items-center justify-center gap-4">
        <p className="text-sm text-[color:var(--color-atlas-muted)]">
          Couldn&rsquo;t load your file map.
        </p>
        <button
          type="button"
          onClick={() => {
            setLoading(true);
            setLoadError(null);
            load();
          }}
          className="rounded-lg border border-[color:var(--color-atlas-border)] px-4 py-2 text-sm text-[color:var(--color-atlas-fg)] hover:border-[color:var(--color-atlas-accent)] transition-colors"
        >
          Try again
        </button>
      </main>
    );
  }

  if (loading || !summary) {
    return (
      <main className="min-h-screen flex items-center justify-center">
        <p className="text-sm text-[color:var(--color-atlas-muted)]">Loading your map...</p>
      </main>
    );
  }

  return (
    <main className="min-h-screen px-6 py-10 max-w-5xl mx-auto">
      <div className="flex items-start justify-between mb-10">
        <div>
          <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
            File Atlas
          </p>
          <h1 className="text-3xl font-semibold mb-1">
            {formatBytes(summary.total_bytes)} across {summary.live_file_count.toLocaleString()}{" "}
            files
          </h1>
          <p className="text-[color:var(--color-atlas-muted)]">
            {summary.live_folder_count.toLocaleString()} folders indexed
          </p>
        </div>
        <button
          type="button"
          onClick={() => setScreen("onboarding")}
          className="rounded-lg border border-[color:var(--color-atlas-border)] px-4 py-2 text-sm text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)] hover:border-[color:var(--color-atlas-accent)] transition-colors"
        >
          Add more folders
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-10">
        <section>
          <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
            By category
          </h2>
          <CategoryBreakdown categories={summary.categories} totalBytes={summary.total_bytes} />
        </section>

        <section>
          <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
            Not touched in a year
          </h2>
          {stale && <StaleBucketCard bucket={stale} />}
        </section>

        <section>
          <TopFilesList
            title="Largest files"
            files={largest}
            valueMode="size"
            emptyLabel="Nothing indexed yet."
          />
        </section>

        <section>
          <TopFilesList
            title="Oldest files"
            files={oldest}
            valueMode="age"
            emptyLabel="Nothing indexed yet."
          />
        </section>
      </div>
    </main>
  );
}
