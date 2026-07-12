import { createContext } from "react";

type Theme = "dark" | "light" | "system";
type ResolvedTheme = Exclude<Theme, "system">;

type ThemeProviderState = {
  theme: Theme;
  setTheme: (theme: Theme) => void;
};

const initialState: ThemeProviderState = {
  theme: "system",
  setTheme: () => null,
};

const ThemeProviderContext = createContext<ThemeProviderState>(initialState);

/**
 * Collapse a `Theme` mode to the concrete light/dark actually in effect,
 * resolving `"system"` against the OS preference. Shared by the provider
 * (which applies it as the `<html>` class) and any consumer that needs the
 * effective mode — e.g. theming a third-party widget that can't read the class.
 */
function resolveMode(mode: Theme): ResolvedTheme {
  if (mode === "system") {
    if (typeof window === "undefined") return "light";
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return mode;
}

export {
  ThemeProviderContext,
  resolveMode,
  type Theme,
  type ResolvedTheme,
  type ThemeProviderState,
};
