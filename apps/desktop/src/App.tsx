import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import HomeView from "./components/HomeView";
import OnboardingWizard from "./components/OnboardingWizard";
import ScanningView from "./components/ScanningView";
import { getHomeSummary } from "./lib/atlas";
import { useScanStore } from "./store/scanStore";
import type { ScanFinishedEvent, ScanProgressEvent } from "./types";

export default function App() {
  const screen = useScanStore((s) => s.screen);
  const setScreen = useScanStore((s) => s.setScreen);
  const setProgress = useScanStore((s) => s.setProgress);
  const lastError = useScanStore((s) => s.lastError);

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

  return (
    <>
      {lastError && (
        <div className="fixed top-0 inset-x-0 z-50 bg-red-500/10 border-b border-red-500/30 text-red-300 text-sm px-6 py-2 text-center">
          {lastError}
        </div>
      )}
      {screen === "loading" && (
        <main className="min-h-screen flex items-center justify-center">
          <p className="text-sm text-[color:var(--color-atlas-muted)]">Starting File Atlas...</p>
        </main>
      )}
      {screen === "onboarding" && <OnboardingWizard />}
      {screen === "scanning" && <ScanningView />}
      {screen === "home" && <HomeView />}
    </>
  );
}
