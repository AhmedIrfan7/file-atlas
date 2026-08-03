import { useEffect } from "react";

import { getCleanupRecommendations, trashSelectedPaths } from "../lib/atlas";
import { bytesForSelection, useCleanupStore } from "../store/cleanupStore";
import DeletePreviewBar from "./DeletePreviewBar";
import RecommendationCard from "./RecommendationCard";

export default function CleanupView() {
  const recommendations = useCleanupStore((s) => s.recommendations);
  const selectedPaths = useCleanupStore((s) => s.selectedPaths);
  const loading = useCleanupStore((s) => s.loading);
  const error = useCleanupStore((s) => s.error);
  const setRecommendations = useCleanupStore((s) => s.setRecommendations);
  const togglePath = useCleanupStore((s) => s.togglePath);
  const setGroupSelected = useCleanupStore((s) => s.setGroupSelected);
  const setLoading = useCleanupStore((s) => s.setLoading);
  const setError = useCleanupStore((s) => s.setError);

  const refresh = () => {
    setLoading(true);
    getCleanupRecommendations()
      .then(setRecommendations)
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleConfirmDelete() {
    const paths = [...selectedPaths];
    if (paths.length === 0) return;
    setLoading(true);
    trashSelectedPaths(paths)
      .then(refresh)
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
  }

  const bytesToFree = bytesForSelection(recommendations, selectedPaths);

  return (
    <main className="min-h-screen px-6 py-10 max-w-3xl mx-auto pb-24">
      <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
        Cleanup
      </p>
      <h1 className="text-2xl font-semibold mb-6">Explainable suggestions, nothing automatic</h1>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      {loading && recommendations.length === 0 ? (
        <p className="text-sm text-[color:var(--color-atlas-muted)]">Looking for suggestions...</p>
      ) : recommendations.length === 0 ? (
        <p className="text-sm text-[color:var(--color-atlas-muted)]">
          Nothing to suggest right now. Your index looks tidy.
        </p>
      ) : (
        <div className="space-y-4">
          {recommendations.map((rec) => (
            <RecommendationCard
              key={rec.kind + rec.title}
              recommendation={rec}
              selectedPaths={selectedPaths}
              onToggleItem={togglePath}
              onToggleAll={(selected) =>
                setGroupSelected(
                  rec.items.map((i) => i.path),
                  selected,
                )
              }
            />
          ))}
        </div>
      )}

      <DeletePreviewBar
        pathCount={selectedPaths.size}
        bytesToFree={bytesToFree}
        paths={[...selectedPaths]}
        onConfirm={handleConfirmDelete}
        busy={loading}
      />
    </main>
  );
}
