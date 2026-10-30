import { defaultSettings, type Conversation } from "../src/types";
import { FakeTransport } from "./fake-transport";

export const CONVERSATION: Conversation = {
  id: "conv-1",
  title: "演示会话",
  created_at_ms: 1,
  updated_at_ms: 2,
};

export const META = {
  edition: "personal",
  version: "0.1.0-mock",
  packages: ["tool.fs", "llm.mock", "agent-loop"],
};

/** A transport that answers the M1 bootstrap commands in-memory. */
export function readyTransport(conversations: Conversation[] = []): FakeTransport {
  const transport = new FakeTransport();
  transport.handle("meta.get", () => META);
  transport.handle("session.list", () => conversations);
  transport.handle("settings.get", () => defaultSettings());
  transport.handle("session.messages", () => []);
  transport.handle("session.create", (payload) => {
    const title = payload["title"];
    return {
      id: "conv-1",
      title: typeof title === "string" ? title : "新会话",
      created_at_ms: 1,
      updated_at_ms: 1,
    };
  });
  transport.handle("chat.send", () => ({ stream_id: "stream-1" }));
  transport.handle("chat.cancel", () => ({}));
  transport.handle("settings.set", (payload) => payload["settings"]);
  transport.handle("approval.decide", () => ({}));
  return transport;
}
