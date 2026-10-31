import "@fontsource/public-sans/400.css";
import "@fontsource/public-sans/500.css";
import "@fontsource/public-sans/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import { Component, StrictMode, useEffect, type ErrorInfo, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./app.css";
import { createTransport } from "./transport";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("missing #root element");
}

/** Marks a successful React mount for the static boot watchdog in index.html. */
function BootSignal(): null {
  useEffect(() => {
    window.__zkBooted = true;
  }, []);
  return null;
}

/**
 * Last-resort boundary: a render/runtime error must show an actionable message,
 * never a blank window (Windows WebView2 white-screen reports).
 */
class StartupErrorBoundary extends Component<{ children: ReactNode }, { message: string | null }> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { message: null };
  }

  static getDerivedStateFromError(error: unknown): { message: string } {
    return { message: error instanceof Error ? `${error.name}: ${error.message}` : String(error) };
  }

  override componentDidCatch(error: unknown, info: ErrorInfo): void {
    console.error("[zk] startup error", error, info.componentStack);
  }

  override render(): ReactNode {
    if (this.state.message !== null) {
      return (
        <div style={{ padding: 24 }}>
          <h1 style={{ fontSize: 16 }}>ZeoKeel 启动失败</h1>
          <pre style={{ whiteSpace: "pre-wrap", wordBreak: "break-word", fontSize: 12 }}>{this.state.message}</pre>
          <p style={{ fontSize: 12 }}>请截图此页面并反馈问题。</p>
        </div>
      );
    }
    return this.props.children;
  }
}

createRoot(container).render(
  <StrictMode>
    <StartupErrorBoundary>
      <BootSignal />
      <App transport={createTransport()} />
    </StartupErrorBoundary>
  </StrictMode>,
);
