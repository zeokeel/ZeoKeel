import { useCallback, useEffect, useState } from "react";
import { api, assertValid, messageOf, validateCreateUser, type User } from "@zeokeel/admin-ui-kit";

export function Users({ onError }: { onError: (message: string) => void }) {
  const [users, setUsers] = useState<User[]>([]);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("employee");
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ users: User[] }>("/api/admin/users").then((r) => { setUsers(r.users); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const create = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => {
      const payload = { username, password, role };
      assertValid(validateCreateUser, payload, "USER");
      await api("/api/admin/users", { method: "POST", body: JSON.stringify(payload) });
      setUsername("");
      setPassword("");
      load();
    })().catch(fail);
  };
  return (
    <section>
      <h2>用户 / 角色</h2>
      <form className="row" onSubmit={create}>
        <input placeholder="username" value={username} onChange={(event) => { setUsername(event.target.value); }} />
        <input placeholder="password (>=8)" type="password" value={password} onChange={(event) => { setPassword(event.target.value); }} />
        <select aria-label="角色" value={role} onChange={(event) => { setRole(event.target.value); }}><option>employee</option><option>auditor</option><option>super_admin</option></select>
        <button type="submit">创建</button>
      </form>
      <table>
        <thead><tr><th>用户名</th><th>角色</th><th>状态</th><th>ID</th></tr></thead>
        <tbody>{users.map((user) => <tr key={user.id}><td>{user.username}</td><td>{user.role}</td><td>{user.status}</td><td className="mono">{user.id.slice(0, 8)}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.users": Users } as const;
