import { useEffect, useRef, useState } from "react";

const DEFAULT_DURATION_MS = 4000;

/** A short-lived success message that clears itself after `durationMs`. */
export function useTransientMessage(durationMs = DEFAULT_DURATION_MS) {
  const [message, setMessage] = useState<string | null>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    },
    [],
  );

  function show(text: string) {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    setMessage(text);
    timeoutRef.current = setTimeout(() => setMessage(null), durationMs);
  }

  return [message, show] as const;
}
