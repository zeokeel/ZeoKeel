import { createContext, useContext, type ReactNode } from "react";
import type {
  AppSettings,
  BootState,
  ChatMessage,
  Conversation,
  Locale,
  Meta,
  ResolvedTheme,
  ThreadLoadState,
  Workspace,
} from "./types";

/** i18n is app infrastructure; packages only receive a translator. */
export type Translate = (key: string, vars?: Record<string, string | number>) => string;

/** Brand values injected by `zk compose` (no package owns a logo). */
export interface BrandConfig {
  name: string;
  poweredBy: string;
  channel: string;
  logoLight: string;
  logoDark: string;
}

/** App actions exposed to UI packages (no package imports the controller). */
export interface UiActions {
  send: (text: string) => void;
  createWorkspace: (path: string, title: string) => void;
  selectWorkspace: (workspaceId: string) => void;
  renameWorkspace: (workspaceId: string, title: string) => void;
  setDefaultWorkspace: (workspaceId: string) => void;
  removeWorkspace: (workspaceId: string, reassignTo?: string) => void;
  cancel: () => void;
  retry: () => void;
  createConversation: () => void;
  selectConversation: (conversationId: string) => void;
  openSettings: () => void;
  closeSettings: () => void;
  saveSettings: (settings: AppSettings) => Promise<void>;
  toggleSidebar: () => void;
  retryBoot: () => void;
  retryMessages: () => void;
  decideApproval: (approvalId: string, decision: "approved" | "rejected") => void;
}

/** Everything a UI package may read; provided by the app shell. */
export interface UiValue {
  t: Translate;
  locale: Locale;
  theme: ResolvedTheme;
  brand: BrandConfig;
  boot: BootState;
  bootError: string | null;
  meta: Meta;
  workspaces: Workspace[];
  activeWorkspaceId: string | null;
  workspaceError: string | null;
  conversationsByWorkspace: Record<string, Conversation[]>;
  /** Conversations of the active workspace (kept for compatibility). */
  conversations: Conversation[];
  activeConversationId: string | null;
  messages: ChatMessage[];
  messagesState: ThreadLoadState;
  messagesError: string | null;
  streaming: boolean;
  offline: boolean;
  settings: AppSettings;
  settingsOpen: boolean;
  sidebarOpen: boolean;
  mock: boolean;
  actions: UiActions;
}

const UiContext = createContext<UiValue | null>(null);

export function UiProvider(props: { value: UiValue; children: ReactNode }) {
  return <UiContext.Provider value={props.value}>{props.children}</UiContext.Provider>;
}

export function useUi(): UiValue {
  const value = useContext(UiContext);
  if (value === null) {
    throw new Error("useUi must be used inside UiProvider");
  }
  return value;
}
