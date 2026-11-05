import { useState, useCallback } from "react";

export default function useLocalStorage<T>(key: string, initialValue: T) {
  const [value, setState] = useState<T>(() => {
    if (typeof window === "undefined") return initialValue;

    try {
      const stored = localStorage.getItem(key);
      return stored !== null ? JSON.parse(stored) : initialValue;
    } catch {
      return initialValue;
    }
  });

  const setValue = useCallback(
    (newValue: T) => {
      localStorage.setItem(key, JSON.stringify(newValue));
      setState(newValue);
    },
    [key]
  );

  return [value, setValue] as const;
}