import { useCallback, useEffect, useState } from "react";
import { adminT, api, apiJson, decodeAdminUsers, messageOf, type AdminUserItem } from "@zeokeel/admin-ui-kit";

export function Users({ onError }: { onError: (message: string) => void }) {
  const [users, setUsers] = useState<AdminUserItem[]>([]);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("employee");
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void apiJson("/api/admin/users", decodeAdminUsers).then((r) => { setUsers(r.users); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const create = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => {
      await api("/api/admin/users", { method: "POST", body: JSON.stringify({ username, password, role }) });
      setUsername("");
      setPassword("");
      load();
    })().catch(fail);
  };
  return (
    <section>
      <h2>{adminT("users.title")}</h2>
      <form className="row" onSubmit={create}>
        <input placeholder="username" value={username} onChange={(event) => { setUsername(event.target.value); }} />
        <input placeholder="password (>=8)" type="password" value={password} onChange={(event) => { setPassword(event.target.value); }} />
        <select aria-label={adminT("common.role")} value={role} onChange={(event) => { setRole(event.target.value); }}><option>employee</option><option>auditor</option><option>super_admin</option></select>
        <button type="submit">{adminT("common.create")}</button>
      </form>
      <table>
        <thead><tr><th>{adminT("users.colUsername")}</th><th>{adminT("common.role")}</th><th>{adminT("common.status")}</th><th>ID</th></tr></thead>
        <tbody>{users.map((user) => <tr key={user.id}><td>{user.username}</td><td>{user.role}</td><td>{user.status}</td><td className="mono">{user.id.slice(0, 8)}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.users": Users } as const;
