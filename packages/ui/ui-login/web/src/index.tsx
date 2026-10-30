/** Enterprise login / first-run page (controlled; auth is provided by the app). */
import { useState } from "react";
import { BrandLogo, Button } from "@zeokeel/ui-kit";

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
  const [serverUrl, setServerUrl] = useState(props.serverUrl ?? "");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [deviceName, setDeviceName] = useState("这台电脑");
  const [showPassword, setShowPassword] = useState(false);
  const needsServer = props.serverUrl === null;
  return (
    <div className="flex h-dvh flex-col items-center justify-center bg-background px-6">
      <div className="w-full max-w-[360px]">
        <div className="flex items-center gap-2"><BrandLogo className="h-8 w-8 rounded-document" /><span className="text-[15px] font-semibold">ZeoKeel</span></div>
        <h1 className="mt-6 text-[18px] font-semibold tracking-[-0.01em]">{needsServer ? "连接企业服务器" : "登录企业版"}</h1>
        <p className="mt-1 text-[12px] leading-[1.55] text-muted-foreground">
          {needsServer ? "首次使用需要填写登录服务地址；保存后可在设置中修改（需重新登录）。" : props.channelLocked ? "使用企业提供的账号登录。" : "使用企业账号登录。"}
        </p>
        <form className="mt-5 grid gap-3" onSubmit={(event) => { event.preventDefault(); props.onSubmit({ ...(needsServer ? { serverUrl } : {}), username, password, deviceName }); }}>
          {needsServer ? (
            <label className="grid gap-1 text-[12px] text-muted-foreground">服务器地址
              <input value={serverUrl} onChange={(e) => setServerUrl(e.target.value)} placeholder="https://zeokeel.example.com" className="rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground" autoComplete="url" />
            </label>
          ) : null}
          <label className="grid gap-1 text-[12px] text-muted-foreground">账号
            <input value={username} onChange={(e) => setUsername(e.target.value)} className="rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground" autoComplete="username" />
          </label>
          <label className="grid gap-1 text-[12px] text-muted-foreground">密码
            <span className="relative block">
              <input type={showPassword ? "text" : "password"} value={password} onChange={(e) => setPassword(e.target.value)} className="w-full rounded-control border border-border bg-background px-2 py-1.5 pe-14 text-[13px] text-foreground" autoComplete="current-password" />
              <button type="button" onClick={() => setShowPassword((v) => !v)} className="absolute end-1 top-1 rounded-sm px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-foreground/[0.05]">{showPassword ? "隐藏" : "显示"}</button>
            </span>
          </label>
          <label className="grid gap-1 text-[12px] text-muted-foreground">设备名
            <input value={deviceName} onChange={(e) => setDeviceName(e.target.value)} className="rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground" />
          </label>
          {props.error !== null ? <p role="alert" className="rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p> : null}
          <Button variant="primary" disabled={props.loading || username.trim() === "" || password === "" || (needsServer && serverUrl.trim() === "")} title={props.loading ? "登录中…" : "登录"}>{props.loading ? "登录中…" : "登录"}</Button>
        </form>
        <p className="mt-4 text-[11px] leading-[1.5] text-muted-foreground">登录即表示同意企业使用条款。密钥与令牌只保存在本机；AI 请求经企业网关。</p>
      </div>
    </div>
  );
}

export const components = { "login.page": LoginPage } as const;
