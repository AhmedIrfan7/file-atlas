import { useEffect } from "react";

import { getStorageMapView } from "../lib/atlas";
import { formatBytes } from "../lib/format";
import { currentPath, useStorageMapStore } from "../store/storageMapStore";
import type { StorageNode } from "../types";
import StorageBreadcrumb from "./StorageBreadcrumb";
import StorageFilters from "./StorageFilters";
import TreemapCanvas from "./TreemapCanvas";

export default function StorageMapView() {
  const breadcrumbs = useStorageMapStore((s) => s.breadcrumbs);
  const category = useStorageMapStore((s) => s.category);
  const sinceDays = useStorageMapStore((s) => s.sinceDays);
  const nodes = useStorageMapStore((s) => s.nodes);
  const totalBytes = useStorageMapStore((s) => s.totalBytes);
  const loading = useStorageMapStore((s) => s.loading);
  const error = useStorageMapStore((s) => s.error);
  const drillInto = useStorageMapStore((s) => s.drillInto);
  const jumpTo = useStorageMapStore((s) => s.jumpTo);
  const setCategory = useStorageMapStore((s) => s.setCategory);
  const setSinceDays = useStorageMapStore((s) => s.setSinceDays);
  const setResult = useStorageMapStore((s) => s.setResult);
  const setLoading = useStorageMapStore((s) => s.setLoading);
  const setError = useStorageMapStore((s) => s.setError);

  const path = currentPath(breadcrumbs);

  useEffect(() => {
    setLoading(true);
    getStorageMapView(path, category, sinceDays)
      .then((resp) => setResult(resp.nodes, resp.total_bytes))
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path, category, sinceDays]);

  function handleDrillInto(node: StorageNode) {
    drillInto(node.path, node.name);
  }

  return (
    <main className="min-h-screen px-6 py-10 max-w-4xl mx-auto">
      <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
        Storage
      </p>
      <h1 className="text-2xl font-semibold mb-1">See where the space actually went</h1>
      <p className="text-sm text-[color:var(--color-atlas-muted)] mb-6">
        {loading ? "Loading..." : `${formatBytes(totalBytes)} in this view`}
      </p>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <StorageBreadcrumb breadcrumbs={breadcrumbs} onJumpTo={jumpTo} />
      <StorageFilters
        category={category}
        sinceDays={sinceDays}
        onCategoryChange={setCategory}
        onSinceDaysChange={setSinceDays}
      />

      {!loading && nodes.length === 0 && breadcrumbs.length === 1 ? (
        <p className="text-sm text-[color:var(--color-atlas-muted)]">
          No completed scans yet. Scan some folders from Home first.
        </p>
      ) : (
        <TreemapCanvas nodes={nodes} onDrillInto={handleDrillInto} />
      )}
    </main>
  );
}
