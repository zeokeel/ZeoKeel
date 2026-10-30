/** Admin shell: login + nav + page slot. Pages are supplied by admin-page packages. */
import { useState } from "react";
import { api, setCsrf, clearCsrf, validateAdminLogin, assertValid } from "@zeokeel/admin-ui-kit";

export type AdminPage = "users" | "wallet" | "pricing" | "audit" | "usage" | "jobs" | "skills" | "connectors" | "settings";
const LABELS: Record<AdminPage, string> = {
  users: "用户/角色", wallet: "钱包", pricing: "价格", audit: "审计", usage: "用量", jobs: "作业", skills: "技能", connectors: "连接器", settings: "系统设置",
};

export function AdminShell(props: { renderPage: (page: AdminPage) => React.ReactNode }) {
  const [loggedIn, setLoggedIn] = useState(false);
  const [page, setPage] = useState<AdminPage>("users");
  if (!loggedIn) return <Login onDone={() => setLoggedIn(true)} />;
  return (
    <div className="admin">
      <aside>
        <h1>ZeoKeel <span>Admin</span></h1>
        {(Object.keys(LABELS) as AdminPage[]).map((item) => (
          <button key={item} className={page === item ? "active" : ""} onClick={() => setPage(item)}>{LABELS[item]}</button>
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
  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    try {
      const payload = { username, password };
      assertValid(validateAdminLogin, payload, "LOGIN");
      const result = await api<{ csrf_token: string }>("/api/admin/auth/login", { method: "POST", body: JSON.stringify(payload) });
      setCsrf(result.csrf_token);
      onDone();
    } catch (cause) { setError(cause instanceof Error ? cause.message : "login failed"); }
  };
  return <form className="login" onSubmit={submit}><h1>ZeoKeel Admin</h1><label>账号<input value={username} onChange={(e) => setUsername(e.target.value)} /></label><label>密码<input type="password" value={password} onChange={(e) => setPassword(e.target.value)} /></label>{error !== "" ? <p className="error">{error}</p> : null}<button type="submit">登录</button></form>;
}

export const components = { "shell.admin": AdminShell } as const;
