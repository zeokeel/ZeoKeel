/** Admin shell: login + nav + page slot. Pages are supplied by admin-page packages. */
import { useState } from "react";
import { api, setCsrf, clearCsrf, validateAdminLogin, assertValid, messageOf } from "@zeokeel/admin-ui-kit";

export type AdminPage = "users" | "wallet" | "pricing" | "audit" | "usage" | "jobs" | "skills" | "connectors" | "apps" | "settings";
const LABELS: Record<AdminPage, string> = {
  users: "用户/角色", wallet: "钱包", pricing: "价格", audit: "审计", usage: "用量", jobs: "作业", skills: "技能", connectors: "连接器", apps: "应用", settings: "系统设置",
};
const PAGES: AdminPage[] = ["users", "wallet", "pricing", "audit", "usage", "jobs", "skills", "connectors", "apps", "settings"];

export function AdminShell(props: { renderPage: (page: AdminPage) => React.ReactNode }) {
  const [loggedIn, setLoggedIn] = useState(false);
  const [page, setPage] = useState<AdminPage>("users");
  if (!loggedIn) return <Login onDone={() => { setLoggedIn(true); }} />;
  return (
    <div className="admin">
      <aside>
        <h1>ZeoKeel <span>Admin</span></h1>
        {PAGES.map((item) => (
          <button key={item} className={page === item ? "active" : ""} onClick={() => { setPage(item); }}>{LABELS[item]}</button>
        ))}
        <button className="logout" onClick={() => { void api("/api/admin/auth/logout", { method: "POST" }).finally(() => { clearCsrf(); setLoggedIn(false); }); }}>退出</button>
      </aside>
      <main>{props.renderPage(page)}</main>
    </div>
  );
}

function Login({ onDone }: { onDone: () => void }) {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const submit = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => {
      const payload = { username, password };
      assertValid(validateAdminLogin, payload, "LOGIN");
      const result = await api<{ csrf_token: string }>("/api/admin/auth/login", { method: "POST", body: JSON.stringify(payload) });
      setCsrf(result.csrf_token);
      onDone();
    })().catch((cause: unknown) => { setError(messageOf(cause)); });
  };
  return <form className="login" onSubmit={submit}><h1>ZeoKeel Admin</h1><label>账号<input value={username} onChange={(event) => { setUsername(event.target.value); }} /></label><label>密码<input type="password" value={password} onChange={(event) => { setPassword(event.target.value); }} /></label>{error !== "" ? <p className="error">{error}</p> : null}<button type="submit">登录</button></form>;
}

export const components = { "shell.admin": AdminShell } as const;
