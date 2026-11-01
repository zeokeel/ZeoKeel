import { createContext, useContext, type ReactNode } from "react";
import type {
  AppSettings,
  ApprovalDecision,
  AttachmentSaveResult,
  BootState,
  ChatMessage,
  CommandDescriptor,
  CompactionMarker,
  Conversation,
  PlanView,
  ConversationExport,
  ConversationOverride,
  FeedbackCategory,
  GoalView,
  Locale,
  Meta,
  ResolvedTheme,
  SessionSearchOutcome,
  SettingsSaveOutcome,
  ThreadLoadState,
  Workspace,
  WorkspaceFileRef,
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
  /** Channel default login server URL (non-empty = locked address, ADR-042). */
  serverUrl?: string;
  appId?: string;
  deepLinkScheme?: string;
  appOriginScheme?: string;
  accent?: string;
  dataDirName?: string;
  productHomeDir?: string;
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
  createConversation: (workspaceId?: string) => Promise<boolean>;
  selectConversation: (conversationId: string) => void;
  openSettings: () => void;
  /** Open settings and deep-link to one `ui.settings.pane` id (validation app-side). */
  openSettingsPane: (paneId: string) => void;
  /** Open settings at the model-related pane (`provider` personal / `enterprise`). */
  openModelSettings: (paneId: string) => void;
  closeSettings: () => void;
  /** Execute one catalog command (the only executor for ⌘K and composer `/`). */
  runCommand: (id: string, query?: string) => void;
  /** Workspace file candidates for `@` (cached `workspace.files`; filtered server-side list). */
  searchWorkspaceFiles: (query: string) => Promise<WorkspaceFileRef[]>;
  /** Save one composer attachment through `workspace.attachment_save` (host validates). */
  saveAttachment: (file: File) => Promise<AttachmentSaveResult>;
  /** Rename/pin/archive one conversation (C18: workspace never changes). */
  updateConversation: (
    conversationId: string,
    patch: { title?: string; pinned?: boolean; archived?: boolean },
  ) => Promise<boolean>;
  /** Fork one conversation inside its workspace and select the copy (C18). */
  forkConversation: (conversationId: string) => Promise<boolean>;
  /** Copy a conversation id to the clipboard (success toast is app-owned). */
  copyConversationId: (conversationId: string) => Promise<boolean>;
  /** Content search in the current workspace (`session.search`); stale responses drop app-side. */
  searchSessions: (query: string, limit?: number) => Promise<SessionSearchOutcome>;
  /** Re-read the active conversation goal (`goal.get`). */
  refreshGoal: () => Promise<void>;
  /** Set/replace the active conversation goal (`goal.set`; UI confirms inline). */
  setGoal: (objective: string, maxRounds?: number) => Promise<boolean>;
  /** Pause/resume/cancel the active conversation goal (`goal.update`). */
  updateGoal: (action: "pause" | "resume" | "cancel") => Promise<boolean>;
  /** Read the active conversation's model/permission override. */
  getConversationOverride: () => Promise<ConversationOverride | null>;
  /** Write the active conversation override (`model`/`reasoning_effort`/`approval_policy`). */
  setConversationOverride: (patch: {
    model?: string;
    reasoning_effort?: string;
    approval_policy?: string;
  }) => Promise<boolean>;
  /** Export the active conversation as Markdown (`session.export`). */
  exportConversation: () => Promise<ConversationExport | null>;
  /** Store local-only feedback JSON (`feedback.submit`; no network). */
  submitFeedback: (category: FeedbackCategory, description: string) => Promise<boolean>;
  /** Compact older history of the active conversation (`session.compact`; noop-safe). */
  compactConversation: () => Promise<boolean>;
  /** Read plan mode/review state of the active conversation (`plan.get`). */
  getPlan: () => Promise<PlanView | null>;
  /** Local plan action (`plan.update`: enter/approve/reject/continue/off; never executes). */
  updatePlan: (action: "enter" | "approve" | "reject" | "continue" | "off", feedback?: string) => Promise<boolean>;
  /** Save the whole settings document under the current revision fence. */
  saveSettings: (settings: AppSettings) => Promise<SettingsSaveOutcome>;
  /** Re-read `settings.get` after a revision conflict; returns the fresh document. */
  reloadSettings: () => Promise<AppSettings>;
  toggleSidebar: () => void;
  /** Open the shell command palette (ui-commands); the palette owns close/Esc. */
  openCommands: () => void;
  retryBoot: () => void;
  retryMessages: () => void;
  decideApproval: (approvalId: string, decision: ApprovalDecision) => void;
  answerQuestion: (questionId: string, answers: unknown) => void;
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
  /** The single app command catalog shared by ⌘K and composer `/`. */
  commands: CommandDescriptor[];
  /** Localized current-model label (personal provider.model / enterprise gateway model). */
  modelLabel: string;
  /** Settings pane id the model label deep-links to (`provider` | `enterprise`). */
  modelSettingsPane: string;
  /** Enterprise gateway model ids cached by the host (personal: empty). */
  gatewayModels: string[];
  /** Active conversation goal (null when unset / not loaded). */
  goal: GoalView | null;
  /** Pending composer `/` command intent (single executor in the app). */
  slashCommand: { name: string; query: string; token: number } | null;
  /** Latest compaction marker of the active conversation (raw messages stay visible). */
  compaction: CompactionMarker | null;
  /** Plan mode + submitted plan review state of the active conversation. */
  plan: PlanView | null;
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
