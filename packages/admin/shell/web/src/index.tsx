/** Admin shell: login + nav + page slot. Pages are supplied by admin-page packages. */
import { useState } from "react";
import { adminT, api, setCsrf, clearCsrf, validateAdminLogin, assertValid, messageOf, type AdminMessageKey } from "@zeokeel/admin-ui-kit";

export type AdminPage = "users" | "wallet" | "pricing" | "audit" | "usage" | "jobs" | "skills" | "connectors" | "apps" | "settings";
const NAV_KEYS: Record<AdminPage, AdminMessageKey> = {
  users: "nav.users",
  wallet: "nav.wallet",
  pricing: "nav.pricing",
  audit: "nav.audit",
  usage: "nav.usage",
  jobs: "nav.jobs",
  skills: "nav.skills",
  connectors: "nav.connectors",
  apps: "nav.apps",
  settings: "nav.settings",
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
          <button key={item} className={page === item ? "active" : ""} onClick={() => { setPage(item); }}>{adminT(NAV_KEYS[item])}</button>
        ))}
        <button className="logout" onClick={() => { void api("/api/admin/auth/logout", { method: "POST" }).finally(() => { clearCsrf(); setLoggedIn(false); }); }}>{adminT("auth.logout")}</button>
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
  return <form className="login" onSubmit={submit}><h1>ZeoKeel Admin</h1><label>{adminT("auth.account")}<input value={username} onChange={(event) => { setUsername(event.target.value); }} /></label><label>{adminT("auth.password")}<input type="password" value={password} onChange={(event) => { setPassword(event.target.value); }} /></label>{error !== "" ? <p className="error">{error}</p> : null}<button type="submit">{adminT("auth.login")}</button></form>;
}

export const components = { "shell.admin": AdminShell } as const;
