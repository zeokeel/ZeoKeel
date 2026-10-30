/**
 * I/O controller: owns the transport, dispatches pure reducer actions, and
 * applies persisted settings to the document (theme/locale).
 */
import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { translate } from "../i18n";
import {
  parseAgentEvent,
  parseConversation,
  parseConversations,
  parseMessages,
  parseMeta,
  parseSettings,
  parseStreamId,
} from "../protocol";
import type { AppSettings, Transport } from "../types";
import { appReducer, createInitialState, type AppState } from "./reducer";

let localIdCounter = 0;

function nextLocalId(prefix: string): string {
  localIdCounter += 1;
  return `${prefix}-${localIdCounter}`;
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

function titleFromText(text: string, fallback: string): string {
  const characters = Array.from(text.trim());
  if (characters.length === 0) {
    return fallback;
  }
  const truncated = characters.length > 24 ? `${characters.slice(0, 24).join("")}…` : text.trim();
  return truncated;
}

export interface AppController {
  state: AppState;
  createConversation: () => void;
  selectConversation: (conversationId: string) => void;
  send: (text: string) => void;
  retrySend: () => void;
  cancel: () => void;
  retryBoot: () => void;
  retryMessages: () => void;
  saveSettings: (settings: AppSettings) => Promise<void>;
  decideApproval: (approvalId: string, decision: "approved" | "rejected") => void;
  setSettingsOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  dismissToast: () => void;
}

export function useAppController(transport: Transport): AppController {
  const [state, dispatch] = useReducer(appReducer, null, createInitialState);
  const [bootAttempt, setBootAttempt] = useState(0);
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [metaRaw, conversationsRaw, settingsRaw] = await Promise.all([
          transport.call("meta.get", {}),
          transport.call("session.list", {}),
          transport.call("settings.get", {}),
        ]);
        if (cancelled) {
          return;
        }
        const conversations = parseConversations(conversationsRaw) ?? [];
        const first = conversations[0];
        dispatch({
          type: "boot_loaded",
          meta: parseMeta(metaRaw),
          conversations,
          settings: parseSettings(settingsRaw),
          activeId: first === undefined ? null : first.id,
        });
      } catch (error: unknown) {
        if (!cancelled) {
          dispatch({ type: "boot_failed", message: errorMessage(error) });
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [transport, bootAttempt]);

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
    if (state.messagesState !== "loading" || state.activeConversationId === null) {
      return;
    }
    const conversationId = state.activeConversationId;
    let cancelled = false;
    void (async () => {
      try {
        const data = await transport.call("session.messages", {
          conversation_id: conversationId,
        });
        if (cancelled) {
          return;
        }
        dispatch({
          type: "messages_loaded",
          conversationId,
          messages: parseMessages(data) ?? [],
        });
      } catch (error: unknown) {
        if (!cancelled) {
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
  }, [state.messagesState, state.activeConversationId, transport]);

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

  const createConversation = useCallback(() => {
    const title = translate(stateRef.current.settings.locale, "conversationNewTitle");
    void (async () => {
      try {
        const data = await transport.call("session.create", { title });
        const conversation = parseConversation(data);
        if (conversation === null) {
          throw new Error("session.create: invalid response");
        }
        dispatch({ type: "conversation_created", conversation });
      } catch (error: unknown) {
        dispatch({ type: "toast", message: errorMessage(error) });
      }
    })();
  }, [transport]);

  const selectConversation = useCallback((conversationId: string) => {
    if (stateRef.current.streaming) {
      return;
    }
    dispatch({ type: "conversation_selected", conversationId });
  }, []);

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
      void (async () => {
        try {
          if (retry) {
            if (current.activeConversationId === null || current.lastSendText === null) {
              return;
            }
            dispatch({ type: "retry_started", now: Date.now() });
            const data = await transport.call("chat.send", {
              conversation_id: current.activeConversationId,
              text: current.lastSendText,
            });
            dispatch({ type: "send_accepted", streamId: parseStreamId(data) ?? "" });
            return;
          }
          let conversationId = current.activeConversationId;
          if (conversationId === null) {
            const created = parseConversation(
              await transport.call("session.create", {
                title: titleFromText(
                  trimmed,
                  translate(current.settings.locale, "conversationNewTitle"),
                ),
              }),
            );
            if (created === null) {
              throw new Error("session.create: invalid response");
            }
            dispatch({ type: "conversation_created", conversation: created });
            conversationId = created.id;
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
            conversation_id: conversationId,
            text: trimmed,
          });
          dispatch({ type: "send_accepted", streamId: parseStreamId(data) ?? "" });
        } catch (error: unknown) {
          dispatch({ type: "send_failed", message: errorMessage(error) });
        }
      })();
    },
    [transport],
  );

  const send = useCallback(
    (text: string) => {
      runSend(text, false);
    },
    [runSend],
  );

  const retrySend = useCallback(() => {
    const current = stateRef.current;
    if (current.lastSendText === null) {
      return;
    }
    runSend(current.lastSendText, true);
  }, [runSend]);

  const cancel = useCallback(() => {
    const streamId = stateRef.current.streamId;
    if (streamId === null || streamId === "") {
      return;
    }
    void (async () => {
      try {
        await transport.call("chat.cancel", { stream_id: streamId });
      } catch (error: unknown) {
        dispatch({ type: "toast", message: errorMessage(error) });
      }
    })();
  }, [transport]);

  const retryBoot = useCallback(() => {
    dispatch({ type: "boot_loading" });
    setBootAttempt((attempt) => attempt + 1);
  }, []);

  const retryMessages = useCallback(() => {
    const conversationId = stateRef.current.activeConversationId;
    if (conversationId === null) {
      return;
    }
    dispatch({ type: "messages_loading", conversationId });
  }, []);

  const saveSettings = useCallback(
    async (next: AppSettings): Promise<void> => {
      const data = await transport.call("settings.set", { settings: next });
      const saved = parseSettings(data);
      dispatch({ type: "settings_saved", settings: saved });
      dispatch({ type: "toast", message: translate(saved.locale, "settingsSaved") });
    },
    [transport],
  );

  const decideApproval = useCallback(
    (approvalId: string, decision: "approved" | "rejected") => {
      void (async () => {
        try {
          await transport.call("approval.decide", {
            approval_id: approvalId,
            decision,
          });
          dispatch({ type: "approval_decided", approvalId, decision });
        } catch (error: unknown) {
          dispatch({ type: "toast", message: errorMessage(error) });
        }
      })();
    },
    [transport],
  );

  const setSettingsOpen = useCallback((open: boolean) => {
    dispatch({ type: "settings_open", open });
  }, []);

  const toggleSidebar = useCallback(() => {
    dispatch({ type: "sidebar_toggled", open: !stateRef.current.sidebarOpen });
  }, []);

  const dismissToast = useCallback(() => {
    dispatch({ type: "toast", message: null });
  }, []);

  return {
    state,
    createConversation,
    selectConversation,
    send,
    retrySend,
    cancel,
    retryBoot,
    retryMessages,
    saveSettings,
    decideApproval,
    setSettingsOpen,
    toggleSidebar,
    dismissToast,
  };
}
