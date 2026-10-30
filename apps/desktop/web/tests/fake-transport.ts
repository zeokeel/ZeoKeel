import type { Transport, UiEvent } from "../src/types";

type Handler = (payload: Record<string, unknown>) => unknown;

export class FakeTransport implements Transport {
  readonly calls: { name: string; payload: Record<string, unknown> }[] = [];
  readonly failures = new Map<string, string>();

  private readonly handlers = new Map<string, Handler>();
  private subscriber: ((event: UiEvent) => void) | null = null;

  handle(name: string, handler: Handler): this {
    this.handlers.set(name, handler);
    return this;
  }

  fail(name: string, message: string): this {
    this.failures.set(name, message);
    return this;
  }

  async call(name: string, payload: Record<string, unknown>): Promise<unknown> {
    this.calls.push({ name, payload });
    const failure = this.failures.get(name);
    if (failure !== undefined) {
      throw new Error(failure);
    }
    const handler = this.handlers.get(name);
    if (handler === undefined) {
      throw new Error(`FakeTransport: no handler for ${name}`);
    }
    return await handler(payload);
  }

  subscribe(handler: (event: UiEvent) => void): () => void {
    this.subscriber = handler;
    return () => {
      this.subscriber = null;
    };
  }

  emit(event: UiEvent): void {
    this.subscriber?.(event);
  }

  emitAgent(streamId: string, seq: number, agentEvent: unknown): void {
    this.emit({ stream_id: streamId, seq, kind: "agent", payload: agentEvent });
  }

  callsFor(name: string): Record<string, unknown>[] {
    return this.calls.filter((call) => call.name === name).map((call) => call.payload);
  }
}
