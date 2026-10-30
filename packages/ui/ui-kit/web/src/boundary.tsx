/** Error boundary for composed slots (docs/11 §8.4 gate 7 / §8.5). */
import { Component, type ErrorInfo, type ReactElement, type ReactNode } from "react";
import { useUi } from "./context";

interface BoundaryProps {
  name: string;
  fallbackText: (name: string) => string;
  retryText: string;
  children: ReactNode;
}

interface BoundaryState {
  failed: boolean;
}

class Boundary extends Component<BoundaryProps, BoundaryState> {
  public override state: BoundaryState = { failed: false };

  public static getDerivedStateFromError(): BoundaryState {
    return { failed: true };
  }

  public override componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error(`[zk] slot failed: ${this.props.name}`, error, info.componentStack);
  }

  public override render(): ReactNode {
    if (!this.state.failed) {
      return this.props.children;
    }
    return (
      <div role="alert" className="flex min-h-0 flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
        <p className="text-[13px] text-destructive">{this.props.fallbackText(this.props.name)}</p>
        <button
          type="button"
          className="rounded-control border border-border px-3 py-1.5 text-[13px] text-foreground hover:bg-muted"
          onClick={() => {
            this.setState({ failed: false });
          }}
        >
          {this.props.retryText}
        </button>
      </div>
    );
  }
}

/** Wraps one composed slot; fallback names the slot so a failure is attributable. */
export function SlotErrorBoundary(props: { name: string; children: ReactNode }): ReactElement {
  const { t } = useUi();
  return (
    <Boundary
      name={props.name}
      fallbackText={(name) => t("slotFailed", { name })}
      retryText={t("slotRetry")}
    >
      {props.children}
    </Boundary>
  );
}
