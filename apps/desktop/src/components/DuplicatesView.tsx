import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import {
  getDuplicateGroups,
  hashDuplicates,
  listRecentActions,
  restoreTrashAction,
  trashSelectedPaths,
} from "../lib/atlas";
import { bytesToFree, pathsToTrash, useDuplicatesStore } from "../store/duplicatesStore";
import { useTransientMessage } from "../lib/useTransientMessage";
import DeletePreviewBar from "./DeletePreviewBar";
import DuplicateGroupCard from "./DuplicateGroupCard";
import RecentActionsPanel from "./RecentActionsPanel";
import Button from "./ui/Button";
import PageHeader from "./ui/PageHeader";
import Panel from "./ui/Panel";

const GROUP_LIMIT = 50;
const RECENT_ACTIONS_LIMIT = 20;

export default function DuplicatesView() {
  const [successMessage, showSuccessMessage] = useTransientMessage();
  const groups = useDuplicatesStore((s) => s.groups);
  const keepOverrides = useDuplicatesStore((s) => s.keepOverrides);
  const hashing = useDuplicatesStore((s) => s.hashing);
  const hashProgress = useDuplicatesStore((s) => s.hashProgress);
  const recentActions = useDuplicatesStore((s) => s.recentActions);
  const loading = useDuplicatesStore((s) => s.loading);
  const error = useDuplicatesStore((s) => s.error);
  const setGroups = useDuplicatesStore((s) => s.setGroups);
  const setKeep = useDuplicatesStore((s) => s.setKeep);
  const setRecentActions = useDuplicatesStore((s) => s.setRecentActions);
  const setLoading = useDuplicatesStore((s) => s.setLoading);
  const setError = useDuplicatesStore((s) => s.setError);

  const refreshGroups = () => {
    getDuplicateGroups(GROUP_LIMIT)
      .then(setGroups)
      .catch((err: unknown) => setError(String(err)));
  };
  const refreshRecentActions = () => {
    listRecentActions(RECENT_ACTIONS_LIMIT)
      .then(setRecentActions)
      .catch((err: unknown) => setError(String(err)));
  };

  useEffect(() => {
    setLoading(true);
    Promise.all([getDuplicateGroups(GROUP_LIMIT), listRecentActions(RECENT_ACTIONS_LIMIT)])
      .then(([g, actions]) => {
        setGroups(g);
        setRecentActions(actions);
      })
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const unlisten = listen("hash-finished", () => {
      refreshGroups();
    });
    return () => {
      void unlisten.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleFindDuplicates() {
    hashDuplicates().catch((err: unknown) => setError(String(err)));
  }

  function handleConfirmDelete() {
    const paths = pathsToTrash(groups, keepOverrides);
    if (paths.length === 0) return;
    setLoading(true);
    trashSelectedPaths(paths)
      .then(() => {
        showSuccessMessage(
          `Sent ${paths.length} item${paths.length === 1 ? "" : "s"} to the Recycle Bin`,
        );
        refreshGroups();
        refreshRecentActions();
      })
      .catch((err: unknown) => setError(String(err)))
      .finally(() => setLoading(false));
  }

  function handleRestore(actionId: number) {
    restoreTrashAction(actionId)
      .then(() => {
        showSuccessMessage("Item restored");
        refreshGroups();
        refreshRecentActions();
      })
      .catch((err: unknown) => setError(String(err)));
  }

  const trashCandidates = pathsToTrash(groups, keepOverrides);
  const freedBytes = bytesToFree(groups, keepOverrides);

  return (
    <main className="min-h-screen px-6 py-10 max-w-3xl mx-auto pb-24">
      <PageHeader
        eyebrow="Duplicates"
        title="Find and safely clean up copies"
        actions={
          <Button onClick={handleFindDuplicates} disabled={hashing}>
            {hashing ? "Scanning..." : "Find duplicates"}
          </Button>
        }
      />

      {hashing && hashProgress && (
        <p className="text-sm text-[color:var(--color-atlas-muted)] mb-6">
          Hashed {hashProgress.filesHashed.toLocaleString()} of{" "}
          {hashProgress.filesTotal.toLocaleString()} candidate files...
        </p>
      )}

      {error && <p className="text-sm text-[color:var(--color-atlas-danger)] mb-4">{error}</p>}
      {successMessage && (
        <p className="text-sm text-[color:var(--color-atlas-success)] mb-4">{successMessage}</p>
      )}

      <section className="mb-10">
        {loading && groups.length === 0 ? (
          <p className="text-sm text-[color:var(--color-atlas-muted)]">Loading...</p>
        ) : groups.length === 0 ? (
          <p className="text-sm text-[color:var(--color-atlas-muted)]">
            No duplicates found yet. Click &ldquo;Find duplicates&rdquo; to hash indexed files and
            check.
          </p>
        ) : (
          <div className="space-y-4">
            {groups.map((group) => (
              <DuplicateGroupCard
                key={group.hash}
                group={group}
                keepOverride={keepOverrides[group.hash]}
                onChangeKeep={(path) => setKeep(group.hash, path)}
              />
            ))}
          </div>
        )}
      </section>

      <Panel className="p-4">
        <h2 className="text-sm font-medium text-[color:var(--color-atlas-muted)] mb-3">
          Recently deleted
        </h2>
        <RecentActionsPanel actions={recentActions} onRestore={handleRestore} />
      </Panel>

      <DeletePreviewBar
        pathCount={trashCandidates.length}
        bytesToFree={freedBytes}
        paths={trashCandidates}
        onConfirm={handleConfirmDelete}
        busy={loading}
      />
    </main>
  );
}
