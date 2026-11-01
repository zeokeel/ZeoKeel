/**
 * I/O controller: owns the transport, dispatches pure reducer actions, and
 * applies persisted settings to the document (theme/locale).
 *
 * Workspace model (docs/26): no workspace = no conversation; every session
 * command carries workspace_id; `chat.send` never auto-creates a conversation.
 */
import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { translate } from "../i18n";
import { validateChatSendResult } from "../validate";
import {
  parseAgentEvent,
  parseConversation,
  parseConversations,
  parseMessages,
  parseMeta,
  parseSettingsDocumentResult,
  parseStreamId,
  parseWorkspace,
  parseWorkspaces,
} from "../protocol";
import type {
  AppSettings,
  ApprovalDecision,
  Conversation,
  SettingsSaveOutcome,
  Transport,
  Workspace,
} from "../types";
import { upgradeInfoOf, type UpgradeInfo } from "../upgrade";
import { appReducer, createInitialState, type AppState } from "./reducer";

let localIdCounter = 0;

function nextLocalId(prefix: string): string {
  localIdCounter += 1;
  return `${prefix}-${String(localIdCounter)}`;
}

function friendlyError(error: unknown, locale: "zh-CN" | "en-US"): string {
  const raw = errorMessage(error);
  const code = raw.split(":", 1)[0]?.trim() ?? "";
  const key = (() => {
    switch (code) {
      case "workspace/path-not-found":
        return "workspaceErrorPathNotFound";
      case "workspace/not-a-directory":
        return "workspaceErrorNotDirectory";
      case "workspace/not-writable":
        return "workspaceErrorNotWritable";
      case "workspace/duplicate-path":
        return "workspaceErrorDuplicate";
      case "workspace/has-sessions":
        return "workspaceErrorHasSessions";
      case "workspace/default":
        return "workspaceErrorDefault";
      case "workspace/reassign-target-invalid":
        return "workspaceErrorReassign";
      case "workspace/confirm-required":
        return "workspaceErrorConfirm";
      case "session/workspace-required":
        return "sessionWorkspaceRequired";
      case "session/workspace-mismatch":
        return "sessionWorkspaceMismatch";
      case "session/not-found":
        return "sessionNotFound";
      default:
        return null;
    }
  })();
  // Keep OS/backend detail in the console for diagnostics; known IPC codes are
  // mapped to localized text, unknown transport errors remain readable.
  console.warn(`[zk] command failed: ${raw}`);
  return key === null ? raw : translate(locale, key);
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "unknown error";
}

export interface AppController {
  state: AppState;
  createConversation: (workspaceId?: string) => Promise<boolean>;
  selectConversation: (conversationId: string) => void;
  /** Re-list the active workspace conversations without changing the selection. */
  refreshActiveConversations: () => Promise<void>;
  /** Surface one localized toast (sessions ops / search errors). */
  showToast: (message: string) => void;
  send: (text: string) => void;
  retrySend: () => void;
  cancel: () => void;
  retryBoot: () => void;
  retryMessages: () => void;
  createWorkspace: (path: string, title: string) => void;
  selectWorkspace: (workspaceId: string) => void;
  renameWorkspace: (workspaceId: string, title: string) => void;
  setDefaultWorkspace: (workspaceId: string) => void;
  removeWorkspace: (workspaceId: string, reassignTo?: string) => void;
  saveSettings: (settings: AppSettings) => Promise<SettingsSaveOutcome>;
  reloadSettings: () => Promise<AppSettings>;
  applyServerUrl: (serverUrl: string) => void;
  decideApproval: (approvalId: string, decision: ApprovalDecision) => void;
  answerQuestion: (questionId: string, answers: unknown) => void;
  setSettingsOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  setCommandsOpen: (open: boolean) => void;
  reportUpgrade: (info: UpgradeInfo) => void;
  dismissToast: () => void;
}

function defaultWorkspace(workspaces: Workspace[]): Workspace | null {
  return workspaces.find((workspace) => workspace.is_default) ?? workspaces[0] ?? null;
}

export function useAppController(transport: Transport): AppController {
  const [state, dispatch] = useReducer(appReducer, null, createInitialState);
  const [bootAttempt, setBootAttempt] = useState(0);
  const stateRef = useRef(state);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const showToast = useCallback((message: string) => {
    dispatch({ type: "toast", message });
  }, []);

  const listWorkspaceConversations = useCallback(
    async (workspaceId: string): Promise<Conversation[]> => {
      const data = await transport.call("session.list", { workspace_id: workspaceId });
      return parseConversations(data) ?? [];
    },
    [transport],
  );

  const refreshConversations = useCallback(
    async (workspaceId: string): Promise<void> => {
      try {
        const conversations = await listWorkspaceConversations(workspaceId);
        const first = conversations[0];
        dispatch({
          type: "conversations_loaded",
          workspaceId,
          conversations,
          activeId: first === undefined ? null : first.id,
        });
      } catch (error: unknown) {
        const message = friendlyError(error, stateRef.current.settings.locale);
        dispatch({ type: "workspace_error", message });
        showToast(message);
      }
    },
    [listWorkspaceConversations, showToast],
  );

  /** Re-list the active workspace conversations (pin/archive/rename/fork) without resetting focus. */
  const refreshActiveConversations = useCallback(async (): Promise<void> => {
    const workspaceId = stateRef.current.activeWorkspaceId;
    if (workspaceId === null) {
      return;
    }
    try {
      const conversations = await listWorkspaceConversations(workspaceId);
      dispatch({ type: "conversations_refreshed", workspaceId, conversations });
    } catch (error: unknown) {
      showToast(friendlyError(error, stateRef.current.settings.locale));
    }
  }, [listWorkspaceConversations, showToast]);

  const reloadAllConversations = useCallback(async (): Promise<Workspace[]> => {
    const workspacesRaw = await transport.call("workspace.list", {});
    const workspaces = parseWorkspaces(workspacesRaw) ?? [];
    const entries = await Promise.all(
      workspaces.map(async (workspace): Promise<[string, Conversation[]]> => {
        try {
          return [workspace.id, await listWorkspaceConversations(workspace.id)];
        } catch {
          return [workspace.id, []];
        }
      }),
    );
    const conversationsByWorkspace: Record<string, Conversation[]> = {};
    for (const [workspaceId, conversations] of entries) {
      conversationsByWorkspace[workspaceId] = conversations;
    }
    dispatch({ type: "workspaces_loaded", workspaces });
    dispatch({ type: "conversations_map_loaded", conversationsByWorkspace });
    return workspaces;
  }, [transport, listWorkspaceConversations]);

  useEffect(() => {
    let cancelled = false;
    const isCancelled = (): boolean => cancelled;
    void (async () => {
      try {
        const [metaRaw, workspacesRaw, settingsRaw] = await Promise.all([
          transport.call("meta.get", {}),
          transport.call("workspace.list", {}),
          transport.call("settings.get", {}),
        ]);
        if (isCancelled()) {
          return;
        }
        const workspaces = parseWorkspaces(workspacesRaw) ?? [];
        const entries = await Promise.all(
          workspaces.map(async (workspace): Promise<[string, Conversation[]]> => {
            return [workspace.id, await listWorkspaceConversations(workspace.id)];
          }),
        );
        if (isCancelled()) {
          return;
        }
        const conversationsByWorkspace: Record<string, Conversation[]> = {};
        for (const [workspaceId, conversations] of entries) {
          conversationsByWorkspace[workspaceId] = conversations;
        }
        const workspace = defaultWorkspace(workspaces);
        const conversations = workspace === null ? [] : (conversationsByWorkspace[workspace.id] ?? []);
        const first = conversations[0];
        const settingsDocument = parseSettingsDocumentResult(settingsRaw);
        dispatch({
          type: "boot_loaded",
          meta: parseMeta(metaRaw),
          workspaces,
          activeWorkspaceId: workspace === null ? null : workspace.id,
          conversationsByWorkspace,
          conversations,
          settings: settingsDocument.settings,
          settingsRevision: settingsDocument.revision,
          activeId: first === undefined ? null : first.id,
        });
      } catch (error: unknown) {
        if (!isCancelled()) {
          const upgrade = upgradeInfoOf(error);
          if (upgrade !== null) {
            dispatch({ type: "upgrade_required", info: upgrade });
          } else {
            dispatch({ type: "boot_failed", message: friendlyError(error, stateRef.current.settings.locale) });
          }
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [transport, bootAttempt, listWorkspaceConversations]);

  useEffect(() => {
    return transport.subscribe((uiEvent) => {
      if (uiEvent.kind !== "agent") {
        console.warn(`[zk] ignored unsupported event kind: ${uiEvent.kind}`);
        return;
      }
      const agentEvent = parseAgentEvent(uiEvent.payload);
      if (agentEvent === null) {
        console.warn("[zk] ignored malformed agent event");
        return;
      }
      dispatch({ type: "agent_event", event: agentEvent, now: Date.now() });
    });
  }, [transport]);

  useEffect(() => {
    const handleOnline = () => {
      dispatch({ type: "offline_changed", offline: false });
    };
    const handleOffline = () => {
      dispatch({ type: "offline_changed", offline: true });
    };
    dispatch({ type: "offline_changed", offline: !navigator.onLine });
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  useEffect(() => {
    if (
      state.messagesState !== "loading" ||
      state.activeConversationId === null ||
      state.activeWorkspaceId === null
    ) {
      return;
    }
    const conversationId = state.activeConversationId;
    const workspaceId = state.activeWorkspaceId;
    let cancelled = false;
    const isCancelled = (): boolean => cancelled;
    void (async () => {
      try {
        const data = await transport.call("session.messages", {
          workspace_id: workspaceId,
          conversation_id: conversationId,
        });
        if (isCancelled()) {
          return;
        }
        dispatch({
          type: "messages_loaded",
          conversationId,
          messages: parseMessages(data) ?? [],
        });
      } catch (error: unknown) {
        if (!isCancelled()) {
          dispatch({
            type: "messages_failed",
            conversationId,
            message: errorMessage(error),
          });
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [state.messagesState, state.activeConversationId, state.activeWorkspaceId, transport]);

  useEffect(() => {
    document.documentElement.lang = state.settings.locale;
    document.documentElement.dataset["theme"] = state.settings.theme;
  }, [state.settings.locale, state.settings.theme]);

  useEffect(() => {
    if (state.toast === null) {
      return;
    }
    const timer = window.setTimeout(() => {
      dispatch({ type: "toast", message: null });
    }, 3200);
    return () => {
      window.clearTimeout(timer);
    };
  }, [state.toast]);

  /**
   * Single conversation-create entry for all three entry points (workspace row
   * `+`, sidebar/header `+`, ⌘K `new-conversation`). One streaming guard +
   * workspace resolution live here (no per-entry duplicates).
   */
  const createConversation = useCallback(
    async (workspaceId?: string): Promise<boolean> => {
      const current = stateRef.current;
      if (current.streaming) {
        showToast(translate(current.settings.locale, "newConversationStreamingBlocked"));
        return false;
      }
      const target = workspaceId ?? current.activeWorkspaceId;
      if (target === null) {
        showToast(translate(current.settings.locale, "workspaceRequired"));
        return false;
      }
      if (!current.workspaces.some((workspace) => workspace.id === target)) {
        showToast(translate(current.settings.locale, "workspaceRequired"));
        return false;
      }
      const title = translate(current.settings.locale, "conversationNewTitle");
      try {
        const data = await transport.call("session.create", {
          workspace_id: target,
          title,
        });
        const conversation = parseConversation(data);
        if (conversation === null) {
          throw new Error("session.create: invalid response");
        }
        if (conversation.workspace_id !== "" && conversation.workspace_id !== target) {
          throw new Error("session.create: workspace mismatch");
        }
        dispatch({ type: "conversation_created", conversation });
        if (target !== stateRef.current.activeWorkspaceId) {
          // Select the target workspace and its new conversation (C18: no cross-workspace leak).
          dispatch({
            type: "conversation_activated",
            workspaceId: target,
            conversationId: conversation.id,
          });
        }
        return true;
      } catch (error: unknown) {
        showToast(friendlyError(error, stateRef.current.settings.locale));
        return false;
      }
    },
    [transport, showToast],
  );

  const selectConversation = useCallback(
    (conversationId: string) => {
      const current = stateRef.current;
      if (conversationId === current.activeConversationId) {
        dispatch({ type: "sidebar_toggled", open: false });
        return;
      }
      let workspaceId: string | null = null;
      for (const [id, conversations] of Object.entries(current.conversationsByWorkspace)) {
        if (conversations.some((conversation) => conversation.id === conversationId)) {
          workspaceId = id;
          break;
        }
      }
      if (workspaceId === null) {
        return;
      }
      if (current.streaming && current.streamId !== null) {
        void transport.call("chat.cancel", {
          workspace_id: current.activeWorkspaceId,
          stream_id: current.streamId,
        });
      }
      if (workspaceId === current.activeWorkspaceId) {
        dispatch({ type: "conversation_selected", conversationId });
      } else {
        dispatch({ type: "conversation_activated", workspaceId, conversationId });
      }
    },
    [transport],
  );

  const selectWorkspace = useCallback(
    (workspaceId: string) => {
      const current = stateRef.current;
      if (workspaceId === current.activeWorkspaceId) {
        dispatch({ type: "sidebar_toggled", open: false });
        return;
      }
      if (current.streaming && current.streamId !== null) {
        void transport.call("chat.cancel", {
          workspace_id: current.activeWorkspaceId,
          stream_id: current.streamId,
        });
      }
      dispatch({ type: "workspace_selected", workspaceId });
      void refreshConversations(workspaceId);
    },
    [refreshConversations, transport],
  );

  const createWorkspace = useCallback(
    (path: string, title: string) => {
      const trimmedPath = path.trim();
      if (trimmedPath === "") {
        dispatch({ type: "workspace_error", message: "workspace/path-required" });
        return;
      }
      void (async () => {
        try {
          const data = await transport.call("workspace.create", {
            path: trimmedPath,
            title: title.trim(),
          });
          const workspace = parseWorkspace(data);
          if (workspace === null) {
            throw new Error("workspace.create: invalid response");
          }
          const wasEmpty = stateRef.current.workspaces.length === 0;
          dispatch({ type: "workspace_created", workspace });
          dispatch({
            type: "conversations_loaded",
            workspaceId: workspace.id,
            conversations: [],
            activeId: null,
          });
          if (wasEmpty) {
            const defaulted = await transport.call("workspace.set_default", {
              workspace_id: workspace.id,
            });
            const list = parseWorkspaces(defaulted);
            if (list !== null) {
              dispatch({ type: "workspaces_loaded", workspaces: list });
            }
          }
          dispatch({ type: "workspace_selected", workspaceId: workspace.id });
        } catch (error: unknown) {
          const message = errorMessage(error);
          dispatch({ type: "workspace_error", message });
          showToast(message);
        }
      })();
    },
    [transport, showToast],
  );

  const renameWorkspace = useCallback(
    (workspaceId: string, title: string) => {
      void (async () => {
        try {
          const data = await transport.call("workspace.rename", {
            workspace_id: workspaceId,
            title,
          });
          const workspace = parseWorkspace(data);
          if (workspace === null) {
            throw new Error("workspace.rename: invalid response");
          }
          const list = stateRef.current.workspaces.map((item) =>
            item.id === workspace.id ? workspace : item,
          );
          dispatch({ type: "workspaces_loaded", workspaces: list });
        } catch (error: unknown) {
          const message = errorMessage(error);
          dispatch({ type: "workspace_error", message });
          showToast(message);
        }
      })();
    },
    [transport, showToast],
  );

  const setDefaultWorkspace = useCallback(
    (workspaceId: string) => {
      void (async () => {
        try {
          const data = await transport.call("workspace.set_default", {
            workspace_id: workspaceId,
          });
          const list = parseWorkspaces(data);
          if (list !== null) {
            dispatch({ type: "workspaces_loaded", workspaces: list });
          }
        } catch (error: unknown) {
          const message = errorMessage(error);
          dispatch({ type: "workspace_error", message });
          showToast(message);
        }
      })();
    },
    [transport, showToast],
  );

  const removeWorkspace = useCallback(
    (workspaceId: string, reassignTo?: string) => {
      void (async () => {
        try {
          const payload: Record<string, unknown> = {
            workspace_id: workspaceId,
            confirm: true,
          };
          if (reassignTo !== undefined && reassignTo !== "") {
            payload["reassign_to"] = reassignTo;
          }
          await transport.call("workspace.remove", payload);
          const activeWorkspace = stateRef.current.activeWorkspaceId;
          const workspaces = await reloadAllConversations();
          if (activeWorkspace === workspaceId) {
            const next = defaultWorkspace(workspaces);
            if (next === null) {
              dispatch({ type: "workspace_selected", workspaceId: "" });
            } else {
              dispatch({ type: "workspace_selected", workspaceId: next.id });
            }
          }
        } catch (error: unknown) {
          const message = errorMessage(error);
          dispatch({ type: "workspace_error", message });
          showToast(message);
        }
      })();
    },
    [transport, reloadAllConversations, showToast],
  );

  const runSend = useCallback(
    (text: string, retry: boolean) => {
      const current = stateRef.current;
      if (current.streaming || current.offline) {
        return;
      }
      const trimmed = text.trim();
      if (trimmed === "") {
        return;
      }
      if (current.activeWorkspaceId === null) {
        showToast(translate(current.settings.locale, "workspaceRequired"));
        return;
      }
      if (current.activeConversationId === null) {
        showToast(translate(current.settings.locale, "conversationRequired"));
        return;
      }
      const workspaceId = current.activeWorkspaceId;
      const conversationId = current.activeConversationId;
      void (async () => {
        try {
          if (retry) {
            if (current.lastSendText === null) {
              return;
            }
            dispatch({ type: "retry_started", now: Date.now() });
            const data = await transport.call("chat.send", {
              workspace_id: workspaceId,
              conversation_id: conversationId,
              text: current.lastSendText,
            });
            if (!validateChatSendResult(data)) {
              dispatch({ type: "send_failed", message: "invalid chat.send response" });
              return;
            }
            dispatch({ type: "send_accepted", streamId: parseStreamId(data) ?? "" });
            return;
          }
          dispatch({
            type: "send_started",
            conversationId,
            text: trimmed,
            messageId: nextLocalId("message"),
            assistantId: nextLocalId("assistant"),
            now: Date.now(),
          });
          const data = await transport.call("chat.send", {
            workspace_id: workspaceId,
            conversation_id: conversationId,
            text: trimmed,
          });
          if (!validateChatSendResult(data)) {
            dispatch({ type: "send_failed", message: "invalid chat.send response" });
            return;
          }
          dispatch({ type: "send_accepted", streamId: parseStreamId(data) ?? "" });
        } catch (error: unknown) {
          dispatch({ type: "send_failed", message: friendlyError(error, stateRef.current.settings.locale) });
        }
      })();
    },
    [transport, showToast],
  );

  const send = useCallback(
    (text: string) => {
      runSend(text, false);
    },
    [runSend],
  );

  const retrySend = useCallback(() => {
    const lastText = stateRef.current.lastSendText;
    if (lastText === null) {
      return;
    }
    runSend(lastText, true);
  }, [runSend]);

  const cancel = useCallback(() => {
    const current = stateRef.current;
    if (current.streamId === null || current.streamId === "") {
      return;
    }
    void (async () => {
      try {
        await transport.call("chat.cancel", {
          workspace_id: current.activeWorkspaceId,
          stream_id: current.streamId,
        });
      } catch (error: unknown) {
        showToast(friendlyError(error, stateRef.current.settings.locale));
      }
    })();
  }, [transport, showToast]);

  const retryBoot = useCallback(() => {
    dispatch({ type: "boot_loading" });
    setBootAttempt((attempt) => attempt + 1);
  }, []);

  const retryMessages = useCallback(() => {
    if (stateRef.current.activeConversationId === null) {
      return;
    }
    dispatch({
      type: "messages_loading",
      conversationId: stateRef.current.activeConversationId,
    });
  }, []);

  const saveSettings = useCallback(
    async (next: AppSettings): Promise<SettingsSaveOutcome> => {
      try {
        const data = await transport.call("settings.set", {
          settings: next,
          expected_revision: stateRef.current.settingsRevision,
        });
        const saved = parseSettingsDocumentResult(data);
        dispatch({
          type: "settings_saved",
          settings: saved.settings,
          revision: saved.revision,
        });
        showToast(translate(saved.settings.locale, "settingsSaved"));
        return { status: "saved", settings: saved.settings, revision: saved.revision };
      } catch (error: unknown) {
        if (errorMessage(error).startsWith("settings/revision-conflict")) {
          // Stale fence: keep the draft, let the dialog offer a reload.
          return { status: "conflict" };
        }
        return {
          status: "error",
          message: friendlyError(error, stateRef.current.settings.locale),
        };
      }
    },
    [transport, showToast],
  );

  const reloadSettings = useCallback(async (): Promise<AppSettings> => {
    const data = await transport.call("settings.get", {});
    const loaded = parseSettingsDocumentResult(data);
    dispatch({ type: "settings_loaded", settings: loaded.settings, revision: loaded.revision });
    return loaded.settings;
  }, [transport]);

  const applyServerUrl = useCallback(
    (serverUrl: string): void => {
      void (async () => {
        try {
          const data = await transport.call("settings.set", {
            settings: { ...stateRef.current.settings, server_url: serverUrl },
            expected_revision: stateRef.current.settingsRevision,
          });
          const saved = parseSettingsDocumentResult(data);
          dispatch({
            type: "settings_saved",
            settings: saved.settings,
            revision: saved.revision,
          });
        } catch {
          // Login already succeeded; persistence is retried on the next settings save.
        }
      })();
    },
    [transport],
  );

  const answerQuestion = useCallback(
    (questionId: string, answers: unknown) => {
      void (async () => {
        try {
          await transport.call("question.answer", { question_id: questionId, answers });
          dispatch({ type: "question_answered", questionId, answers });
        } catch (error: unknown) {
          showToast(friendlyError(error, stateRef.current.settings.locale));
        }
      })();
    },
    [transport, showToast],
  );

  const decideApproval = useCallback(
    (approvalId: string, decision: ApprovalDecision) => {
      void (async () => {
        try {
          await transport.call("approval.decide", {
            approval_id: approvalId,
            decision,
          });
          dispatch({ type: "approval_decided", approvalId, decision });
        } catch (error: unknown) {
          showToast(friendlyError(error, stateRef.current.settings.locale));
        }
      })();
    },
    [transport, showToast],
  );

  const setSettingsOpen = useCallback((open: boolean) => {
    dispatch({ type: "settings_open", open });
  }, []);

  const toggleSidebar = useCallback(() => {
    dispatch({ type: "sidebar_toggled", open: !stateRef.current.sidebarOpen });
  }, []);

  const setCommandsOpen = useCallback((open: boolean) => {
    dispatch({ type: "commands_open", open });
  }, []);

  const reportUpgrade = useCallback((info: UpgradeInfo) => {
    dispatch({ type: "upgrade_required", info });
  }, []);

  const dismissToast = useCallback(() => {
    dispatch({ type: "toast", message: null });
  }, []);

  return {
    state,
    createConversation,
    selectConversation,
    refreshActiveConversations,
    showToast,
    send,
    retrySend,
    cancel,
    retryBoot,
    retryMessages,
    createWorkspace,
    selectWorkspace,
    renameWorkspace,
    setDefaultWorkspace,
    removeWorkspace,
    saveSettings,
    reloadSettings,
    applyServerUrl,
    decideApproval,
    answerQuestion,
    setSettingsOpen,
    toggleSidebar,
    setCommandsOpen,
    reportUpgrade,
    dismissToast,
  };
}
