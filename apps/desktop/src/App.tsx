import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import AiSearchView from "./components/AiSearchView";
import CleanupView from "./components/CleanupView";
import DuplicatesView from "./components/DuplicatesView";
import HomeView from "./components/HomeView";
import NavBar from "./components/NavBar";
import OnboardingWizard from "./components/OnboardingWizard";
import ScanningView from "./components/ScanningView";
import SearchView from "./components/SearchView";
import StorageMapView from "./components/StorageMapView";
import TimelineView from "./components/TimelineView";
import { getAiStatus, getHomeSummary } from "./lib/atlas";
import { useAiStore } from "./store/aiStore";
import { useDuplicatesStore } from "./store/duplicatesStore";
import { useScanStore } from "./store/scanStore";
import type {
  EmbedFinishedEvent,
  EmbedProgressEvent,
  HashFinishedEvent,
  HashProgressEvent,
  ScanFinishedEvent,
  ScanProgressEvent,
} from "./types";

export default function App() {
  const screen = useScanStore((s) => s.screen);
  const setScreen = useScanStore((s) => s.setScreen);
  const setProgress = useScanStore((s) => s.setProgress);
  const lastError = useScanStore((s) => s.lastError);
  const setError = useScanStore((s) => s.setError);
  const setHashing = useDuplicatesStore((s) => s.setHashing);
  const setHashProgress = useDuplicatesStore((s) => s.setHashProgress);
  const setEmbedProgress = useAiStore((s) => s.setEmbedProgress);
  const setAiStatus = useAiStore((s) => s.setStatus);

  useEffect(() => {
    getHomeSummary()
      .then((summary) => {
        setScreen(summary.live_file_count > 0 ? "home" : "onboarding");
      })
      .catch(() => setScreen("onboarding"));
  }, [setScreen]);

  useEffect(() => {
    const unlistenProgress = listen<ScanProgressEvent>("scan-progress", (event) => {
      setProgress({
        currentRoot: event.payload.root,
        filesSeen: event.payload.files_seen,
        bytesSeen: event.payload.bytes_seen,
      });
    });
    const unlistenFinished = listen<ScanFinishedEvent>("scan-finished", () => {
      setScreen("home");
    });
    return () => {
      void unlistenProgress.then((f) => f());
      void unlistenFinished.then((f) => f());
    };
  }, [setProgress, setScreen]);

  useEffect(() => {
    const unlistenHashProgress = listen<HashProgressEvent>("hash-progress", (event) => {
      setHashing(true);
      setHashProgress({
        filesHashed: event.payload.files_hashed,
        filesTotal: event.payload.files_total,
      });
    });
    const unlistenHashFinished = listen<HashFinishedEvent>("hash-finished", () => {
      setHashing(false);
      setHashProgress(null);
    });
    return () => {
      void unlistenHashProgress.then((f) => f());
      void unlistenHashFinished.then((f) => f());
    };
  }, [setHashing, setHashProgress]);

  useEffect(() => {
    const unlistenEmbedProgress = listen<EmbedProgressEvent>("embed-progress", (event) => {
      setEmbedProgress({
        filesEmbedded: event.payload.files_embedded,
        filesTotal: event.payload.files_total,
      });
    });
    const unlistenEmbedFinished = listen<EmbedFinishedEvent>("embed-finished", () => {
      setEmbedProgress(null);
      getAiStatus()
        .then(setAiStatus)
        .catch(() => undefined);
    });
    return () => {
      void unlistenEmbedProgress.then((f) => f());
      void unlistenEmbedFinished.then((f) => f());
    };
  }, [setEmbedProgress, setAiStatus]);

  return (
    <>
      {lastError && (
        <div className="fixed top-0 inset-x-0 z-50 flex items-center justify-center gap-4 bg-[color:var(--color-atlas-danger)]/10 border-b border-[color:var(--color-atlas-danger)]/30 text-[color:var(--color-atlas-danger)] text-sm px-6 py-2">
          <span>{lastError}</span>
          <button
            type="button"
            onClick={() => setError(null)}
            aria-label="Dismiss error"
            className="shrink-0 text-[color:var(--color-atlas-danger)]/70 hover:text-[color:var(--color-atlas-danger)]"
          >
            &times;
          </button>
        </div>
      )}
      {screen === "loading" && (
        <main className="min-h-screen flex items-center justify-center">
          <p className="text-sm text-[color:var(--color-atlas-muted)]">Starting File Atlas...</p>
        </main>
      )}
      {screen === "onboarding" && <OnboardingWizard />}
      {screen === "scanning" && <ScanningView />}
      {(screen === "home" ||
        screen === "search" ||
        screen === "duplicates" ||
        screen === "cleanup" ||
        screen === "storage" ||
        screen === "timeline" ||
        screen === "ai") && (
        <>
          <NavBar />
          {screen === "home" && <HomeView />}
          {screen === "search" && <SearchView />}
          {screen === "duplicates" && <DuplicatesView />}
          {screen === "cleanup" && <CleanupView />}
          {screen === "storage" && <StorageMapView />}
          {screen === "timeline" && <TimelineView />}
          {screen === "ai" && <AiSearchView />}
        </>
      )}
    </>
  );
}
