import { useEffect, useState } from "react";
import type { ResolvedTheme, Theme } from "./types";

function systemPrefersDark(): boolean {
  if (typeof window.matchMedia !== "function") {
    return false;
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** Resolve `auto` against the OS preference and keep it live for BrandLogo. */
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
