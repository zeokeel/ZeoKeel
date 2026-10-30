import { useEffect, useState } from "react";
import { api, type User } from "@zeokeel/admin-ui-kit";

export function Users({ onError }: { onError: (message: string) => void }) {
  const [users, setUsers] = useState<User[]>([]);
  const [username, setUsername] = useState(""); const [password, setPassword] = useState(""); const [role, setRole] = useState("employee");
  const load = () => { void api<{ users: User[] }>("/api/admin/users").then((r) => setUsers(r.users)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const create = async (event: React.FormEvent) => { event.preventDefault(); try { await api("/api/admin/users", { method: "POST", body: JSON.stringify({ username, password, role }) }); setUsername(""); setPassword(""); load(); } catch (e) { onError((e as Error).message); } };
  return <section><h2>用户 / 角色</h2><form className="row" onSubmit={create}><input placeholder="username" value={username} onChange={(e) => setUsername(e.target.value)} /><input placeholder="password (>=8)" type="password" value={password} onChange={(e) => setPassword(e.target.value)} /><select aria-label="角色" value={role} onChange={(e) => setRole(e.target.value)}><option>employee</option><option>auditor</option><option>super_admin</option></select><button type="submit">创建</button></form><table><thead><tr><th>用户名</th><th>角色</th><th>状态</th><th>ID</th></tr></thead><tbody>{users.map((user) => <tr key={user.id}><td>{user.username}</td><td>{user.role}</td><td>{user.status}</td><td className="mono">{user.id.slice(0, 8)}</td></tr>)}</tbody></table></section>;
}


export const components = { "page.users": Users } as const;
