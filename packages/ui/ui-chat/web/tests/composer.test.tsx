/** @vitest-environment jsdom */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import {
  defaultSettings,
  UiProvider,
  type AttachmentSaveResult,
  type ChatMessage,
  type CommandDescriptor,
  type DraftRef,
  type UiActions,
  type UiValue,
  type WorkspaceFileRef,
} from "@zeokeel/ui-kit";
import {
  ATTACHMENT_MAX_BYTES,
  ATTACHMENT_MAX_COUNT,
  ChatView,
  REFERENCE_BLOCK_MARKER,
  composeOutgoingText,
  composerUsage,
  detectComposerTrigger,
  parseUserContent,
  validateAttachment,
} from "../src/index";

afterEach(() => {
  cleanup();
});

beforeAll(() => {
  if (typeof globalThis.ResizeObserver === "undefined") {
    class ResizeObserverMock {
      observe(): void {
        void 0;
      }
      unobserve(): void {
        void 0;
      }
      disconnect(): void {
        void 0;
      }
    }
    Object.defineProperty(globalThis, "ResizeObserver", {
      writable: true,
      value: ResizeObserverMock,
    });
  }
  if (typeof window.matchMedia !== "function") {
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        dispatchEvent: () => false,
      }),
    });
  }
});

const COMMANDS: CommandDescriptor[] = [
  { id: "slash-file", name: "file", title: "Add file", group: "Add", kind: "action", surface: "slash" },
  { id: "slash-model", name: "model", title: "Choose model", group: "Commands", kind: "popupSelect", surface: "slash" },
  { id: "slash-compact", name: "compact", title: "Compact (Batch 2)", group: "Commands", kind: "disabled", surface: "slash", disabled: true },
];

const FILES: WorkspaceFileRef[] = [
  { path: "demo-data/hello.txt", kind: "file" },
  { path: "demo-data", kind: "dir" },
];

function t(key: string): string {
  return key;
}

function makeActions(overrides: Partial<UiActions> = {}): UiActions {
  const noop = (): void => undefined;
  return {
    send: noop,
    createWorkspace: noop,
    selectWorkspace: noop,
    renameWorkspace: noop,
    setDefaultWorkspace: noop,
    removeWorkspace: noop,
    cancel: noop,
    retry: noop,
    createConversation: noop,
    selectConversation: noop,
    openSettings: noop,
    openSettingsPane: noop,
    openModelSettings: noop,
    closeSettings: noop,
    runCommand: noop,
    searchWorkspaceFiles: (): Promise<WorkspaceFileRef[]> => Promise.resolve([]),
    saveAttachment: (): Promise<AttachmentSaveResult> =>
      Promise.resolve({ path: ".zeokeel/attachments/c/f.png", name: "f.png", bytes: 4, mime: "image/png" }),
    updateConversation: (): Promise<boolean> => Promise.resolve(true),
    forkConversation: (): Promise<boolean> => Promise.resolve(true),
    copyConversationId: (): Promise<boolean> => Promise.resolve(true),
    searchSessions: (): Promise<{ items: []; truncated: boolean }> => Promise.resolve({ items: [], truncated: false }),
    refreshGoal: (): Promise<void> => Promise.resolve(),
    setGoal: (): Promise<boolean> => Promise.resolve(true),
    updateGoal: (): Promise<boolean> => Promise.resolve(true),
    getConversationOverride: (): Promise<null> => Promise.resolve(null),
    setConversationOverride: (): Promise<boolean> => Promise.resolve(true),
    exportConversation: (): Promise<null> => Promise.resolve(null),
    submitFeedback: (): Promise<boolean> => Promise.resolve(true),
    saveSettings: (settings): Promise<{ status: "saved"; settings: typeof settings; revision: number }> =>
      Promise.resolve({ status: "saved", settings, revision: 1 }),
    reloadSettings: (): Promise<ReturnType<typeof defaultSettings>> =>
      Promise.resolve(defaultSettings()),
    toggleSidebar: noop,
    openCommands: noop,
    retryBoot: noop,
    retryMessages: noop,
    decideApproval: noop,
    answerQuestion: noop,
    ...overrides,
  };
}

function makeUi(overrides: Partial<UiValue> = {}, actions: Partial<UiActions> = {}): UiValue {
  return {
    t,
    locale: "zh-CN",
    theme: "light",
    brand: { name: "ZeoKeel", poweredBy: "show", channel: "official", logoLight: "", logoDark: "" },
    boot: "ready",
    bootError: null,
    meta: { edition: "personal", version: "0.1.0", packages: [] },
    workspaces: [],
    activeWorkspaceId: "ws-1",
    workspaceError: null,
    conversationsByWorkspace: {},
    conversations: [],
    activeConversationId: "conv-1",
    messages: [],
    messagesState: "idle",
    messagesError: null,
    streaming: false,
    offline: false,
    settings: defaultSettings(),
    settingsOpen: false,
    sidebarOpen: false,
    mock: false,
    commands: COMMANDS,
    modelLabel: "test-model",
    modelSettingsPane: "provider",
    gatewayModels: [],
    goal: null,
    slashCommand: null,
    actions: makeActions(actions),
    ...overrides,
  };
}

function renderChat(ui: UiValue): ReturnType<typeof render> {
  return render(
    <UiProvider value={ui}>
      <ChatView />
    </UiProvider>,
  );
}

function composer(): HTMLTextAreaElement {
  const element = screen.getByRole("textbox", { name: "composerAria" });
  if (!(element instanceof HTMLTextAreaElement)) {
    throw new Error("composer is not a textarea");
  }
  return element;
}

describe("composer reference block (compose/parse)", () => {
  const refs: DraftRef[] = [
    { kind: "workspace", path: "demo-data/hello.txt" },
    { kind: "workspace", path: "demo-data/" },
    { kind: "attachment", path: ".zeokeel/attachments/c/a.png", status: "ready" },
  ];

  it("appends the normative trailing block and round-trips it", () => {
    const text = composeOutgoingText("read these", refs);
    expect(text).toBe(
      `read these\n\n${REFERENCE_BLOCK_MARKER}\n@demo-data/hello.txt\n@demo-data/\n@.zeokeel/attachments/c/a.png`,
    );
    const parsed = parseUserContent(text);
    expect(parsed.body).toBe("read these");
    expect(parsed.refs.map((ref) => ref.path)).toEqual([
      "demo-data/hello.txt",
      "demo-data/",
      ".zeokeel/attachments/c/a.png",
    ]);
  });

  it("supports reference-only messages and leaves ref-less bodies untouched", () => {
    expect(composeOutgoingText("", refs.slice(0, 1))).toBe(
      `${REFERENCE_BLOCK_MARKER}\n@demo-data/hello.txt`,
    );
    expect(composeOutgoingText("plain body", [])).toBe("plain body");
    expect(parseUserContent("plain body")).toEqual({ body: "plain body", refs: [] });
  });

  it("de-duplicates and never emits invalid/unsafe paths", () => {
    const text = composeOutgoingText("x", [
      { kind: "workspace", path: "a.txt" },
      { kind: "workspace", path: "a.txt" },
      { kind: "workspace", path: "../escape.txt" },
      { kind: "workspace", path: "/abs.txt" },
      { kind: "workspace", path: "bad\npath" },
      { kind: "workspace", path: "dir/" },
    ]);
    expect(text).toBe(`x\n\n${REFERENCE_BLOCK_MARKER}\n@a.txt\n@dir/`);
  });

  it("skips uploading/error attachments (never send half-baked files)", () => {
    const text = composeOutgoingText("x", [
      { kind: "attachment", path: ".zeokeel/a.png", status: "uploading" },
      { kind: "attachment", path: ".zeokeel/b.png", status: "error" },
      { kind: "workspace", path: "ok.txt" },
    ]);
    expect(text).toBe(`x\n\n${REFERENCE_BLOCK_MARKER}\n@ok.txt`);
  });

  it("decodes @@-leading paths and ignores fake markers inside the body", () => {
    const parsed = parseUserContent(`hello\n\n${REFERENCE_BLOCK_MARKER}\n@@mention.txt`);
    expect(parsed.refs.map((ref) => ref.path)).toEqual(["@mention.txt"]);
    const fake = parseUserContent(`look:\n\n${REFERENCE_BLOCK_MARKER}\nthis is not a reference`);
    expect(fake.refs).toEqual([]);
    expect(fake.body).toContain("this is not a reference");
    const nul = parseUserContent(`${REFERENCE_BLOCK_MARKER}\n@bad\u0000path\n@ok.txt`);
    expect(nul.refs.map((ref) => ref.path)).toEqual(["ok.txt"]);
  });
});

describe("composer triggers", () => {
  it("detects word-start / and @ only", () => {
    expect(detectComposerTrigger("/", 1)).toMatchObject({ kind: "command", query: "" });
    expect(detectComposerTrigger("hi /set", 7)).toMatchObject({ kind: "command", query: "set" });
    expect(detectComposerTrigger("hi @demo", 8)).toMatchObject({ kind: "file", query: "demo" });
    expect(detectComposerTrigger("http://x", 9)).toBeNull();
    expect(detectComposerTrigger("word@x", 6)).toBeNull();
  });
});

describe("attachment validation", () => {
  it("rejects oversize/empty/unknown/count-overflow", () => {
    expect(validateAttachment(new File([new Uint8Array(0)], "a.txt"), 0)).toBe("attachmentEmpty");
    expect(
      validateAttachment(new File([new Uint8Array(ATTACHMENT_MAX_BYTES + 1)], "a.png"), 0),
    ).toBe("attachmentTooLarge");
    expect(validateAttachment(new File(["x"], "a.exe"), 0)).toBe("attachmentTypeNotAllowed");
    expect(validateAttachment(new File(["x"], "a.txt"), ATTACHMENT_MAX_COUNT)).toBe(
      "attachmentLimit",
    );
    expect(validateAttachment(new File(["x"], "a.txt"), 0)).toBeNull();
  });
});

describe("composer usage", () => {
  const message = (usage: NonNullable<ChatMessage["usage"]>): ChatMessage => ({
    id: "m1",
    role: "assistant",
    content: "",
    created_at_ms: 1,
    status: "done",
    toolCalls: [],
    approvals: [],
    questions: [],
    error: null,
    usage,
  });

  it("sums persisted usage and reports the latest turn", () => {
    const usage = composerUsage([
      message({ input_tokens: 10, output_tokens: 5, total_tokens: 15, cached_tokens: 2, reasoning_tokens: 0 }),
      message({ input_tokens: 7, output_tokens: 3, total_tokens: 10, cached_tokens: 1, reasoning_tokens: 0 }),
    ]);
    expect(usage.turn).toMatchObject({ input_tokens: 7, total_tokens: 10 });
    expect(usage.session).toMatchObject({ input_tokens: 17, output_tokens: 8, total_tokens: 25 });
  });

  it("returns null (usage-not-reported) when no usage exists", () => {
    expect(composerUsage([])).toEqual({ turn: null, session: null });
  });
});

describe("composer surface", () => {
  it("/ opens the DSH slash catalog, filters, runs and clears the trigger", async () => {
    const runCommand = vi.fn();
    renderChat(makeUi({}, { runCommand }));
    const input = composer();
    await userEvent.type(input, "/model");
    const list = await screen.findByRole("listbox", { name: "commandMenuLabel" });
    expect(list).toBeTruthy();
    expect(screen.getByRole("option", { name: /Choose model/ })).toBeTruthy();
    await userEvent.keyboard("{Enter}");
    expect(runCommand).toHaveBeenCalledWith("slash-model");
    expect(input.value).toBe("");
    await userEvent.keyboard("{Escape}");
    expect(document.activeElement).toBe(input);
    expect(screen.queryByRole("listbox", { name: "commandMenuLabel" })).toBeNull();
  });

  it("shows Batch 2 placeholders as disabled (never executed) and Esc keeps the draft", async () => {
    const runCommand = vi.fn();
    renderChat(makeUi({}, { runCommand }));
    const input = composer();
    await userEvent.type(input, "/compact");
    const option = await screen.findByRole("option", { name: /Compact/ });
    expect(option).toBeTruthy();
    expect(option instanceof HTMLButtonElement && option.disabled).toBe(true);
    await userEvent.keyboard("{Enter}");
    expect(runCommand).not.toHaveBeenCalled();
    await expect(screen.findByText("commandUnimplemented")).resolves.toBeTruthy();
    await userEvent.keyboard("x");
    // Draft is preserved with the remaining typed body.
    expect(input.value.length).toBeGreaterThan(0);
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(input.value).toContain("/compact");
  });

  it("@ lists workspace files and adds a de-duplicated reference chip", async () => {
    const searchWorkspaceFiles = vi.fn((): Promise<WorkspaceFileRef[]> => Promise.resolve(FILES));
    renderChat(makeUi({}, { searchWorkspaceFiles }));
    const input = composer();
    await userEvent.type(input, "@demo");
    const option = await screen.findByRole("option", { name: /demo-data\/hello\.txt/ });
    await userEvent.click(option);
    expect(input.value).toBe("");
    const list = screen.getByRole("list", { name: "composerRefsLabel" });
    expect(list.textContent).toContain("hello.txt");
    // Selecting the directory reference is allowed but not a duplicate of the file.
    await userEvent.type(input, "@demo-data");
    const menu2 = await screen.findByRole("listbox", { name: "fileMenuLabel" });
    expect(menu2.textContent).toContain("demo-data");
  });

  it("Enter sends (Shift+Enter newlines) and IME composing never sends", async () => {
    const send = vi.fn();
    renderChat(makeUi({}, { send }));
    const input = composer();
    await userEvent.type(input, "hello");
    await userEvent.keyboard("{Shift>}{Enter}{/Shift}");
    await userEvent.type(input, "world");
    expect(input.value).toBe("hello\nworld");
    expect(send).not.toHaveBeenCalled();
    // IME: composition + Enter must not submit.
    fireEvent.compositionStart(input);
    fireEvent.keyDown(input, { key: "Enter", code: "Enter", isComposing: true });
    expect(send).not.toHaveBeenCalled();
    fireEvent.compositionEnd(input);
    await userEvent.keyboard("{Enter}");
    expect(send).toHaveBeenCalledWith("hello\nworld");
    expect(input.value).toBe("");
  });

  it("attachments upload through the app action and render a data: thumbnail", async () => {
    const saveAttachment = vi.fn(
      (): Promise<AttachmentSaveResult> =>
        Promise.resolve({ path: ".zeokeel/attachments/conv-1/abc-pic.png", name: "pic.png", bytes: 4, mime: "image/png" }),
    );
    const send = vi.fn();
    const { container } = renderChat(makeUi({}, { saveAttachment, send }));
    const fileInput = container.querySelector('input[type="file"]');
    if (!(fileInput instanceof HTMLInputElement)) {
      throw new Error("file input missing");
    }
    const png = new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], "pic.png", { type: "image/png" });
    fireEvent.change(fileInput, { target: { files: [png] } });
    expect(await screen.findByText("pic.png")).toBeTruthy();
    const thumbnail = screen.getByRole("img", { name: "attachmentImageAlt" });
    expect(thumbnail.getAttribute("src")?.startsWith("data:")).toBe(true);
    const input = composer();
    await userEvent.type(input, "see attached");
    await waitFor(() => {
      expect(
        (() => {
          const button = screen.getByRole("button", { name: "send" });
          return button instanceof HTMLButtonElement && !button.disabled;
        })(),
      ).toBe(true);
    });
    await userEvent.click(screen.getByRole("button", { name: "send" }));
    expect(send).toHaveBeenCalledWith(
      `see attached\n\n${REFERENCE_BLOCK_MARKER}\n@.zeokeel/attachments/conv-1/abc-pic.png`,
    );
  });

  it("model label deep-links to the model settings pane", async () => {
    const openModelSettings = vi.fn();
    renderChat(makeUi({ modelLabel: "test-model", modelSettingsPane: "provider" }, { openModelSettings }));
    await userEvent.click(screen.getByRole("button", { name: "modelLabelAria" }));
    expect(openModelSettings).toHaveBeenCalledWith("provider");
  });

  it("usage status shows reported usage and the unreported fallback", () => {
    const withUsage = makeUi({
      messages: [
        {
          id: "m1",
          role: "assistant",
          content: "hi",
          created_at_ms: 1,
          status: "done",
          toolCalls: [],
          approvals: [],
          questions: [],
          error: null,
          usage: { input_tokens: 10, output_tokens: 5, total_tokens: 15, cached_tokens: 0, reasoning_tokens: 0 },
        },
      ],
    });
    const first = renderChat(withUsage);
    const status = screen.getByRole("status", { name: /usageLabel/ });
    expect(status.textContent).toContain("usageTurnCompact");
    expect(status.getAttribute("aria-label")).toContain("usageTurnLine");
    first.unmount();
    renderChat(makeUi());
    expect(screen.getByRole("status", { name: "usageUnreported" }).textContent).toBe(
      "usageUnreported",
    );
  });
});
