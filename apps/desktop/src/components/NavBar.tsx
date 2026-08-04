import { useScanStore } from "../store/scanStore";
import type { ScreenState } from "../store/scanStore";

const TABS: { screen: ScreenState; label: string }[] = [
  { screen: "home", label: "Home" },
  { screen: "search", label: "Search" },
  { screen: "duplicates", label: "Duplicates" },
  { screen: "cleanup", label: "Cleanup" },
  { screen: "storage", label: "Storage" },
  { screen: "timeline", label: "Timeline" },
  { screen: "ai", label: "AI Search" },
];

export default function NavBar() {
  const screen = useScanStore((s) => s.screen);
  const setScreen = useScanStore((s) => s.setScreen);

  return (
    <nav className="flex items-center gap-1 border-b border-[color:var(--color-atlas-border)] px-6 py-3">
      {TABS.map((tab) => (
        <button
          key={tab.screen}
          type="button"
          onClick={() => setScreen(tab.screen)}
          className={`rounded-md px-3 py-1.5 text-sm transition-colors ${
            screen === tab.screen
              ? "bg-white/10 text-[color:var(--color-atlas-fg)]"
              : "text-[color:var(--color-atlas-muted)] hover:text-[color:var(--color-atlas-fg)]"
          }`}
        >
          {tab.label}
        </button>
      ))}
    </nav>
  );
}
