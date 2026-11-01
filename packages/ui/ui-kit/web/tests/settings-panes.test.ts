import { describe, expect, it } from "vitest";
import type {
  SettingsErrors,
  SettingsForm,
  SettingsPaneAppProps,
  SettingsPaneContribution,
  SettingsPaneProps,
} from "../src/index";

const EMPTY_ERRORS: SettingsErrors = {
  baseUrl: null,
  model: null,
  maxSteps: null,
  temperature: null,
  maxTokens: null,
  topP: null,
  seed: null,
  toolChoiceTools: null,
  scanSeconds: null,
  toolTimeoutMs: null,
  approvalTimeoutMs: null,
  embeddingBaseUrl: null,
  embeddingDims: null,
  mediaBaseUrl: null,
  webSearchBaseUrl: null,
  webSearchTimeoutMs: null,
};

const EMPTY_FORM: SettingsForm = {
  providerKind: "mock",
  baseUrl: "",
  model: "mock",
  apiKey: "",
  theme: "auto",
  locale: "zh-CN",
  autoApproveReadOnly: true,
  serverUrl: "",
  maxSteps: "50",
  temperature: "",
  maxTokens: "",
  topP: "",
  seed: "",
  stop: "",
  toolChoiceMode: "auto",
  toolChoiceTools: "",
  parallelToolCalls: "default",
  reasoningEffort: "",
  promptCache: "",
  memoryEnabled: true,
  memoryDir: "",
  hooksEnabled: true,
  hooksLog: "",
  scheduleEnabled: false,
  scanSeconds: "10",
  toolTimeoutMs: "120000",
  approvalTimeoutMs: "300000",
  embeddingBaseUrl: "",
  embeddingApiKey: "",
  embeddingModel: "",
  embeddingDims: "",
  embeddingDir: "",
  mediaBaseUrl: "",
  mediaApiKey: "",
  mediaImageModel: "",
  mediaAudioModel: "",
  mediaTranscriptionModel: "",
  mediaRerankModel: "",
  webSearchBaseUrl: "",
  webSearchTimeoutMs: "15000",
};

const APP: SettingsPaneAppProps = {
  serverUrlLocked: false,
  gatewayModels: [],
  gatewayModel: "",
  updateText: null,
  updateBusy: false,
};

/** A pane only needs the shell-provided props; no app or edition import. */
function GeneralPane(props: SettingsPaneProps): string {
  return `${props.t("settingsPaneGeneral")}:${props.draft.theme}:${String(props.saving)}`;
}

describe("settings pane shared types", () => {
  it("accepts a pane contribution with the shared component contract", () => {
    const contribution: SettingsPaneContribution = {
      id: "general",
      order: 10,
      Component: GeneralPane,
    };
    expect(contribution.id).toBe("general");
    expect(contribution.order).toBe(10);
    const props: SettingsPaneProps = {
      t: (key: string) => key,
      draft: EMPTY_FORM,
      update: (): void => undefined,
      errors: EMPTY_ERRORS,
      saving: false,
      app: APP,
    };
    expect(contribution.Component).toBe(GeneralPane);
    expect(GeneralPane(props)).toBe("settingsPaneGeneral:auto:false");
  });
});
