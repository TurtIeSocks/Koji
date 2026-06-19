import { useEffect, useMemo } from "react";
import { useStore } from "shadmin-core";

import {
  ThemeProviderContext,
  type ResolvedTheme,
  type Theme,
} from "@/lib/theme-context";

interface ThemeProviderProps {
  children: React.ReactNode;
  /**
   * Initial mode when no user preference has been persisted yet. Named
   * `defaultTheme` for backwards compatibility — it configures the *mode*
   * (light/dark/system), not a palette.
   */
  defaultTheme?: Theme;
  storageKey?: string;
}

function resolveMode(mode: Theme): ResolvedTheme {
  if (mode === "system") {
    if (typeof window === "undefined") return "light";
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return mode;
}

/**
 * Theme provider that manages the active light/dark mode.
 *
 * The mode is persisted via ra-core's `useStore` and applied as a class
 * (`light`/`dark`) on `<html>` so Tailwind `dark:` variants and the shadcn
 * `.dark` token block resolve. Theme *palettes* are no longer a runtime
 * concern — tokens live in CSS (`src/index.css`, the `src/styles/themes/*`
 * palette files, and the registry `cssVars`).
 *
 * @internal
 */
function ThemeProvider({
  children,
  defaultTheme = "system",
  storageKey = "theme",
}: ThemeProviderProps) {
  const [mode, setMode] = useStore<Theme>(storageKey, defaultTheme);
  const effectiveMode = resolveMode(mode);

  useEffect(() => {
    const root = window.document.documentElement;
    root.classList.remove("light", "dark");
    root.classList.add(effectiveMode);
  }, [effectiveMode]);

  const value = useMemo(
    () => ({ theme: mode, setTheme: setMode }),
    [mode, setMode],
  );

  return (
    <ThemeProviderContext.Provider value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}

export { ThemeProvider };
