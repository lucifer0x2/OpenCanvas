import { useState, useCallback } from "react";

export default function useLocalStorage<T>(
  key: string,
  initialValue: T,
  validate: (value: unknown) => value is T,
) {
  const [value, setState] = useState<T>(() => {
    if (typeof window === "undefined") return initialValue;

    try {
      const stored = localStorage.getItem(key);
      const parsed: unknown = stored !== null ? JSON.parse(stored) : undefined;
      return validate(parsed) ? parsed : initialValue;
    } catch {
      return initialValue;
    }
  });

  const setValue = useCallback(
    (newValue: T) => {
      setState(newValue);
      // Preference persistence is best effort; blocked/quota-full storage must not break UI.
      try {
        localStorage.setItem(key, JSON.stringify(newValue));
      } catch {
        // Keep the preference in memory for this tab.
      }
    },
    [key],
  );

  return [value, setValue] as const;
}
