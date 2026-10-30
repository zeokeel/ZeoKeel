import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(() => {
  cleanup();
});

// jsdom does not implement ResizeObserver; Radix primitives (tooltip/dialog) probe it.
if (typeof globalThis.ResizeObserver === "undefined") {
  class ResizeObserverMock {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  Object.defineProperty(globalThis, "ResizeObserver", {
    writable: true,
    value: ResizeObserverMock,
  });
}

if (typeof window.matchMedia !== "function") {
  const matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  });
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: matchMedia,
  });
}
