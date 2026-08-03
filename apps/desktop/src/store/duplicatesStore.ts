import { create } from "zustand";

import type { ActionRow, DuplicateGroup } from "../types";

interface HashProgressState {
  filesHashed: number;
  filesTotal: number;
}

interface DuplicatesStore {
  groups: DuplicateGroup[];
  // Overrides the backend's suggested keeper: hash -> chosen keep path.
  keepOverrides: Record<string, string>;
  hashing: boolean;
  hashProgress: HashProgressState | null;
  recentActions: ActionRow[];
  loading: boolean;
  error: string | null;
  setGroups: (groups: DuplicateGroup[]) => void;
  setKeep: (hash: string, path: string) => void;
  setHashing: (hashing: boolean) => void;
  setHashProgress: (progress: HashProgressState | null) => void;
  setRecentActions: (actions: ActionRow[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useDuplicatesStore = create<DuplicatesStore>((set) => ({
  groups: [],
  keepOverrides: {},
  hashing: false,
  hashProgress: null,
  recentActions: [],
  loading: false,
  error: null,
  setGroups: (groups) => set({ groups }),
  setKeep: (hash, path) =>
    set((state) => ({ keepOverrides: { ...state.keepOverrides, [hash]: path } })),
  setHashing: (hashing) => set({ hashing }),
  setHashProgress: (hashProgress) => set({ hashProgress }),
  setRecentActions: (recentActions) => set({ recentActions }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
}));

/** The path to keep for a group: the user's override, or the backend's suggestion. */
export function keepPathFor(group: DuplicateGroup, overrides: Record<string, string>): string {
  return (
    overrides[group.hash] ??
    group.members.find((m) => m.suggested_keep)?.file.path ??
    group.members[0].file.path
  );
}

/** Every path across every group that would be trashed given current overrides. */
export function pathsToTrash(
  groups: DuplicateGroup[],
  overrides: Record<string, string>,
): string[] {
  return groups.flatMap((group) => {
    const keep = keepPathFor(group, overrides);
    return group.members.filter((m) => m.file.path !== keep).map((m) => m.file.path);
  });
}

export function bytesToFree(groups: DuplicateGroup[], overrides: Record<string, string>): number {
  return groups.reduce((total, group) => {
    const keep = keepPathFor(group, overrides);
    const freed = group.members.filter((m) => m.file.path !== keep).length;
    return total + freed * group.size_bytes;
  }, 0);
}
