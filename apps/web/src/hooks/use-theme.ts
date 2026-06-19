import { use } from "react";

import {
  type ResolvedTheme,
  ThemeProviderContext,
  type Theme,
} from "@/lib/theme-context";

/**
 * Returns the current theme mode and a setter as a `[mode, setMode]` tuple.
 *
 * Mirrors the upstream `ra-ui-materialui` `useTheme` contract.
 *
 * @example
 * const [mode, setMode] = useTheme();
 */
export function useTheme(): [Theme, (theme: Theme) => void] {
  const context = use(ThemeProviderContext);

  if (context === undefined)
    throw new Error("useTheme must be used within a ThemeProvider");

  return [context.theme, context.setTheme];
}
/**
 * Resolves `'dark' | 'light' | 'system'` to `'dark' | 'light'`
 * @returns 'dark' | 'light'
 */
export function useResolvedTheme(): ResolvedTheme {
  const [theme] = useTheme();
  return resolveTheme(theme);
}

function resolveTheme(theme: Theme): ResolvedTheme {
  return theme === "system"
    ? typeof window !== "undefined" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light"
    : theme;
}
