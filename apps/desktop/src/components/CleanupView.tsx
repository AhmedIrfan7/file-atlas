import { useEffect } from "react";

import {
  getCleanupRecommendations,
  listRecentActions,
  restoreTrashAction,
  trashSelectedPaths,
} from "../lib/atlas";
import { bytesForSelection, useCleanupStore } from "../store/cleanupStore";
import { useTransientMessage } from "../lib/useTransientMessage";
import DeletePreviewBar from "./DeletePreviewBar";
import RecentActionsPanel from "./RecentActionsPanel";
import RecommendationCard from "./RecommendationCard";

const RECENT_ACTIONS_LIMIT = 20;

export default function CleanupView() {
  const [successMessage, showSuccessMessage] = useTransientMessage();
  const recommendations = useCleanupStore((s) => s.recommendations);
  const selectedPaths = useCleanupStore((s) => s.selectedPaths);
  const recentActions = useCleanupStore((s) => s.recentActions);
  const loading = useCleanupStore((s) => s.loading);
  const error = useCleanupStore((s) => s.error);
  const setRecommendations = useCleanupStore((s) => s.setRecommendations);
  const refreshRecommendations = useCleanupStore((s) => s.refreshRecommendations);
  const setRecentActions = useCleanupStore((s) => s.setRecentActions);
  const togglePath = useCleanupStore((s) => s.togglePath);
  const setGroupSelected = useCleanupStore((s) => s.setGroupSelected);
  const setLoading = useCleanupStore((s) => s.setLoading);
  const setError = useCleanupStore((s) => s.setError);

  const refreshRecentActions = () => {
    listRecentActions(RECENT_ACTIONS_LIMIT)
      .then(setRecentActions)
      .catch((err: unknown) => setError(String(err)));
  };

  useEffect(() => {
    setLoading(true);
    Promise.all([getCleanupRecommendations(), listRecentActions(RECENT_ACTIONS_LIMIT)])
      .then(([recs, actions]) => {
        setRecommendations(recs);
        setRecentActions(actions);
      })
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleConfirmDelete() {
    const paths = [...selectedPaths];
    if (paths.length === 0) return;
    setLoading(true);
    trashSelectedPaths(paths)
      .then(() => {
        showSuccessMessage(
          `Sent ${paths.length} item${paths.length === 1 ? "" : "s"} to the Recycle Bin`,
        );
        refreshRecentActions();
        return getCleanupRecommendations().then(refreshRecommendations);
      })
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
  }

  function handleRestore(actionId: number) {
    restoreTrashAction(actionId)
      .then(() => {
        showSuccessMessage("Item restored");
        refreshRecentActions();
        return getCleanupRecommendations().then(refreshRecommendations);
      })
      .catch((err: unknown) => setError(String(err)));
  }

  const bytesToFree = bytesForSelection(recommendations, selectedPaths);

  return (
    <main className="min-h-screen px-6 py-10 max-w-3xl mx-auto pb-24">
      <p className="text-xs uppercase tracking-widest text-[color:var(--color-atlas-muted)] mb-2">
        Cleanup
      </p>
      <h1 className="text-2xl font-semibold mb-6">Explainable suggestions, nothing automatic</h1>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}
      {successMessage && <p className="text-sm text-emerald-400 mb-4">{successMessage}</p>}

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

      <section className="mt-10">
        <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
          Recently deleted
        </h2>
        <RecentActionsPanel actions={recentActions} onRestore={handleRestore} />
      </section>

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
