import { create } from "zustand";

import type { Burst, Granularity, TimelineBucket } from "../types";

export interface TimelineView {
  label: string;
  granularity: Granularity;
  sinceDays: number | null;
}

export const TIMELINE_VIEWS: TimelineView[] = [
  { label: "This week", granularity: "day", sinceDays: 7 },
  { label: "This year", granularity: "month", sinceDays: 365 },
  { label: "All time", granularity: "month", sinceDays: null },
];

interface TimelineStore {
  view: TimelineView;
  buckets: TimelineBucket[];
  screenshotBursts: Burst[];
  projectBursts: Burst[];
  loading: boolean;
  error: string | null;
  setView: (view: TimelineView) => void;
  setBuckets: (buckets: TimelineBucket[]) => void;
  setBursts: (screenshotBursts: Burst[], projectBursts: Burst[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useTimelineStore = create<TimelineStore>((set) => ({
  view: TIMELINE_VIEWS[0],
  buckets: [],
  screenshotBursts: [],
  projectBursts: [],
  loading: false,
  error: null,
  setView: (view) => set({ view }),
  setBuckets: (buckets) => set({ buckets }),
  setBursts: (screenshotBursts, projectBursts) => set({ screenshotBursts, projectBursts }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));
