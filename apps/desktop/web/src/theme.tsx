import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import type { Theme } from "./types";

export type ResolvedTheme = "light" | "dark";

const ThemeContext = createContext<ResolvedTheme>("light");

function systemPrefersDark(): boolean {
  if (typeof window.matchMedia !== "function") {
    return false;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** Resolve `auto` against the OS preference and keep it live. */
export function useResolvedTheme(theme: Theme): ResolvedTheme {
  const [systemDark, setSystemDark] = useState(systemPrefersDark);

  useEffect(() => {
    if (typeof window.matchMedia !== "function") {
      return;
    }
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (event: MediaQueryListEvent) => {
      setSystemDark(event.matches);
    };
    query.addEventListener("change", handler);
    return () => {
      query.removeEventListener("change", handler);
    };
  }, []);

  return theme === "dark" || (theme === "auto" && systemDark) ? "dark" : "light";
}

export function ThemeProvider(props: { theme: ResolvedTheme; children: ReactNode }) {
  return <ThemeContext.Provider value={props.theme}>{props.children}</ThemeContext.Provider>;
}

export function useThemeResolution(): ResolvedTheme {
  return useContext(ThemeContext);
}
