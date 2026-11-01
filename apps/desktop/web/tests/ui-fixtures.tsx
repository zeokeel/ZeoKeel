import {
  defaultSettings,
  type AppSettings,
  type AttachmentSaveResult,
  type Conversation,
  type SettingsSaveOutcome,
  type UiActions,
  type UiValue,
  type Workspace,
  type WorkspaceFileRef,
} from "@zeokeel/ui-kit";
import { translate } from "../src/i18n";

const noop = (): void => undefined;

export const testWorkspace: Workspace = {
  id: "ws-home",
  path: "/home/test",
  title: "主目录",
  is_default: true,
  created_at_ms: 1,
  updated_at_ms: 1,
};

const testConversation: Conversation = {
  id: "conv-1",
  workspace_id: testWorkspace.id,
  title: "演示会话",
  created_at_ms: 1,
  updated_at_ms: 1,
};

export const testBrand = {
  name: "ZeoKeel",
  poweredBy: "show",
  channel: "official",
  logoLight: "./brand/logo.svg",
  logoDark: "./brand/logo-dark.svg",
};

export interface TestUiOverrides extends Partial<Omit<UiValue, "actions">> {
  actions?: Partial<UiActions>;
}

/** Minimal UiProvider value for unit tests of UI packages. */
export function testUi(overrides: TestUiOverrides = {}): UiValue {
  const base: UiValue = {
    t: (key, vars) => translate("zh-CN", key, vars),
    locale: "zh-CN",
    theme: "light",
    brand: testBrand,
    boot: "ready",
    bootError: null,
    meta: { edition: "personal", version: "0.1.0", packages: [] },
    workspaces: [testWorkspace],
    activeWorkspaceId: testWorkspace.id,
    workspaceError: null,
    conversationsByWorkspace: { [testWorkspace.id]: [testConversation] },
    conversations: [testConversation],
    activeConversationId: null,
    messages: [],
    messagesState: "idle",
    messagesError: null,
    streaming: false,
    offline: false,
    settings: defaultSettings(),
    settingsOpen: false,
    sidebarOpen: false,
    mock: false,
    commands: [],
    modelLabel: "mock",
    modelSettingsPane: "provider",
    actions: {
      send: noop,
      cancel: noop,
      retry: noop,
      createConversation: noop,
      createWorkspace: noop,
      selectWorkspace: noop,
      renameWorkspace: noop,
      setDefaultWorkspace: noop,
      removeWorkspace: noop,
      selectConversation: noop,
      openSettings: noop,
      openSettingsPane: noop,
      openModelSettings: noop,
      runCommand: noop,
      searchWorkspaceFiles: (): Promise<WorkspaceFileRef[]> => Promise.resolve([]),
      saveAttachment: (): Promise<AttachmentSaveResult> => Promise.reject(new Error("not configured")),
      updateConversation: (): Promise<boolean> => Promise.resolve(true),
      forkConversation: (): Promise<boolean> => Promise.resolve(true),
      copyConversationId: (): Promise<boolean> => Promise.resolve(true),
      searchSessions: (): Promise<{ items: []; truncated: boolean }> => Promise.resolve({ items: [], truncated: false }),
      closeSettings: noop,
      saveSettings: (settings: AppSettings): Promise<SettingsSaveOutcome> =>
        Promise.resolve({ status: "saved", settings, revision: 1 }),
      reloadSettings: (): Promise<AppSettings> => Promise.resolve(defaultSettings()),
      toggleSidebar: noop,
      openCommands: noop,
      retryBoot: noop,
      retryMessages: noop,
      decideApproval: noop,
      answerQuestion: noop,
    },
  };
  return {
    ...base,
    ...overrides,
    actions: { ...base.actions, ...(overrides.actions ?? {}) },
  };
}
