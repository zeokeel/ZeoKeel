/** Enterprise login / first-run page (controlled; auth is provided by the app). */
import { useState } from "react";
import { BrandLogo, Button, useUi } from "@zeokeel/ui-kit";

export interface LoginInput {
  serverUrl?: string;
  username: string;
  password: string;
  deviceName: string;
}

export interface LoginPageProps {
  serverUrl: string | null;
  channelLocked: boolean;
  loading: boolean;
  error: string | null;
  onSubmit: (input: LoginInput) => void;
}

export function LoginPage(props: LoginPageProps) {
  const { t, brand } = useUi();
  const [serverUrl, setServerUrl] = useState(props.serverUrl ?? "");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [deviceName, setDeviceName] = useState(t("loginDeviceDefault"));
  const [showPassword, setShowPassword] = useState(false);
  const needsServer = props.serverUrl === null;
  const submit = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    props.onSubmit({ ...(needsServer ? { serverUrl } : {}), username, password, deviceName });
  };
  return (
    <div className="flex h-dvh flex-col items-center justify-center bg-background px-6">
      <div className="w-full max-w-[360px]">
        <div className="flex items-center gap-2">
          <BrandLogo className="h-8 w-8 rounded-document" />
          <span className="text-[15px] font-semibold">{brand.name}</span>
        </div>
        <h1 className="mt-6 text-[18px] font-semibold tracking-[-0.01em]">
          {needsServer ? t("loginTitleFirstRun") : t("loginTitle")}
        </h1>
        <p className="mt-1 text-[12px] leading-[1.55] text-muted-foreground">
          {needsServer ? t("loginHintFirstRun") : props.channelLocked ? t("loginHintLocked") : t("loginHint")}
        </p>
        <form className="mt-5 grid gap-3" onSubmit={submit} noValidate>
          {needsServer ? (
            <label className="grid gap-1 text-[12px] text-muted-foreground">
              {t("loginServerAddress")}
              <input
                value={serverUrl}
                onChange={(event) => { setServerUrl(event.target.value); }}
                placeholder="https://zeokeel.example.com"
                className="rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground"
                autoComplete="url"
                aria-label={t("loginServerAddress")}
              />
            </label>
          ) : null}
          <label className="grid gap-1 text-[12px] text-muted-foreground">
            {t("loginUsername")}
            <input
              value={username}
              onChange={(event) => { setUsername(event.target.value); }}
              className="rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground"
              autoComplete="username"
              aria-label={t("loginUsername")}
            />
          </label>
          <label className="grid gap-1 text-[12px] text-muted-foreground">
            {t("loginPassword")}
            <span className="relative block">
              <input
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(event) => { setPassword(event.target.value); }}
                className="w-full rounded-control border border-border bg-background px-2 py-1.5 pe-14 text-[13px] text-foreground"
                autoComplete="current-password"
                aria-label={t("loginPassword")}
              />
              <button
                type="button"
                onClick={() => { setShowPassword((value) => !value); }}
                className="absolute end-1 top-1 rounded-sm px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-foreground/[0.05]"
              >
                {showPassword ? t("loginHide") : t("loginShow")}
              </button>
            </span>
          </label>
          <label className="grid gap-1 text-[12px] text-muted-foreground">
            {t("loginDeviceName")}
            <input
              value={deviceName}
              onChange={(event) => { setDeviceName(event.target.value); }}
              className="rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground"
              aria-label={t("loginDeviceName")}
            />
          </label>
          {props.error !== null ? (
            <p role="alert" className="rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p>
          ) : null}
          <Button
            variant="primary"
            disabled={props.loading || username.trim() === "" || password === "" || (needsServer && serverUrl.trim() === "")}
          >
            {props.loading ? t("loginSubmitting") : t("loginSubmit")}
          </Button>
        </form>
        <p className="mt-4 text-[11px] leading-[1.5] text-muted-foreground">{t("loginTerms")}</p>
      </div>
    </div>
  );
}

export const components = { "login.page": LoginPage } as const;
