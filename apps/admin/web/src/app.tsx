import { useEffect, useState } from "react";

interface User { id: string; username: string; display_name: string; role: string; status: string }
interface Wallet { user_id: string; username: string; balance_micros: number }
interface Price { model: string; input_per_1k_micros: number; output_per_1k_micros: number; enabled: boolean }

let csrf = "";
async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/json");
  if (csrf !== "") headers.set("x-csrf-token", csrf);
  const response = await fetch(path, { ...init, headers, credentials: "include" });
  const body = (await response.json()) as { ok?: boolean; error?: string } & T;
  if (!response.ok || body.ok === false) throw new Error(body.error ?? `HTTP ${response.status}`);
  return body;
}

export function AdminApp() {
  const [loggedIn, setLoggedIn] = useState(false);
  const [page, setPage] = useState<"users" | "wallet" | "pricing" | "audit" | "usage" | "jobs" | "settings">("users");
  const [error, setError] = useState("");
  if (!loggedIn) return <Login onDone={() => { setLoggedIn(true); setError(""); }} />;
  return (
    <div className="admin">
      <aside>
        <h1>ZeoKeel <span>Admin</span></h1>
        {(["users", "wallet", "pricing", "audit", "usage", "jobs", "settings"] as const).map((item) => (
          <button key={item} className={page === item ? "active" : ""} onClick={() => setPage(item)}>{item === "users" ? "用户/角色" : item === "wallet" ? "钱包" : item === "pricing" ? "价格" : item === "audit" ? "审计" : item === "usage" ? "用量" : item === "jobs" ? "作业" : "系统设置"}</button>
        ))}
        <button className="logout" onClick={() => { void api("/api/admin/auth/logout", { method: "POST" }).finally(() => { csrf = ""; setLoggedIn(false); }); }}>退出</button>
      </aside>
      <main>
        {error !== "" ? <p className="error">{error}</p> : null}
        {page === "users" ? <Users onError={setError} /> : null}
        {page === "wallet" ? <Wallets onError={setError} /> : null}
        {page === "pricing" ? <Pricing onError={setError} /> : null}
        {page === "audit" ? <AuditLog onError={setError} /> : null}
        {page === "usage" ? <Usage onError={setError} /> : null}
        {page === "jobs" ? <Jobs onError={setError} /> : null}
        {page === "settings" ? <Settings onError={setError} /> : null}
      </main>
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
      const result = await api<{ csrf_token: string }>("/api/admin/auth/login", { method: "POST", body: JSON.stringify({ username, password }) });
      csrf = result.csrf_token;
      onDone();
    } catch (cause) { setError(cause instanceof Error ? cause.message : "login failed"); }
  };
  return <form className="login" onSubmit={submit}><h1>ZeoKeel Admin</h1><label>账号<input value={username} onChange={(e) => setUsername(e.target.value)} /></label><label>密码<input type="password" value={password} onChange={(e) => setPassword(e.target.value)} /></label>{error !== "" ? <p className="error">{error}</p> : null}<button type="submit">登录</button></form>;
}

function Users({ onError }: { onError: (message: string) => void }) {
  const [users, setUsers] = useState<User[]>([]);
  const [username, setUsername] = useState(""); const [password, setPassword] = useState(""); const [role, setRole] = useState("employee");
  const load = () => { void api<{ users: User[] }>("/api/admin/users").then((r) => setUsers(r.users)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const create = async (event: React.FormEvent) => { event.preventDefault(); try { await api("/api/admin/users", { method: "POST", body: JSON.stringify({ username, password, role }) }); setUsername(""); setPassword(""); load(); } catch (e) { onError((e as Error).message); } };
  return <section><h2>用户 / 角色</h2><form className="row" onSubmit={create}><input placeholder="username" value={username} onChange={(e) => setUsername(e.target.value)} /><input placeholder="password (>=8)" type="password" value={password} onChange={(e) => setPassword(e.target.value)} /><select aria-label="角色" value={role} onChange={(e) => setRole(e.target.value)}><option>employee</option><option>auditor</option><option>super_admin</option></select><button type="submit">创建</button></form><table><thead><tr><th>用户名</th><th>角色</th><th>状态</th><th>ID</th></tr></thead><tbody>{users.map((user) => <tr key={user.id}><td>{user.username}</td><td>{user.role}</td><td>{user.status}</td><td className="mono">{user.id.slice(0, 8)}</td></tr>)}</tbody></table></section>;
}

function Wallets({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<Wallet[]>([]); const [delta, setDelta] = useState("1000000");
  const load = () => { void api<{ items: Wallet[] }>("/api/admin/wallet").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const adjust = async (userId: string) => { try { await api(`/api/admin/wallet/${userId}/adjust`, { method: "POST", body: JSON.stringify({ delta_micros: Number(delta), reason: "admin ui", idempotency_key: `ui-${userId}-${Date.now()}` }) }); load(); } catch (e) { onError((e as Error).message); } };
  return <section><h2>钱包</h2><p className="hint">调账会写 ledger；余额由账本派生。</p><label>调账金额（micros）<input value={delta} onChange={(e) => setDelta(e.target.value)} /></label><table><thead><tr><th>用户</th><th>余额</th><th>操作</th></tr></thead><tbody>{items.map((item) => <tr key={item.user_id}><td>{item.username}</td><td>{item.balance_micros}</td><td><button onClick={() => void adjust(item.user_id)}>调账</button></td></tr>)}</tbody></table></section>;
}

function Pricing({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<Price[]>([]); const [model, setModel] = useState("deepseek-chat"); const [input, setInput] = useState("1000"); const [output, setOutput] = useState("2000");
  const load = () => { void api<{ items: Price[] }>("/api/admin/pricing").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const save = async (event: React.FormEvent) => { event.preventDefault(); try { await api(`/api/admin/pricing/${model}`, { method: "PUT", body: JSON.stringify({ input_per_1k_micros: Number(input), output_per_1k_micros: Number(output) }) }); load(); } catch (e) { onError((e as Error).message); } };
  return <section><h2>模型价格</h2><form className="row" onSubmit={save}><input value={model} onChange={(e) => setModel(e.target.value)} /><input value={input} onChange={(e) => setInput(e.target.value)} /><input value={output} onChange={(e) => setOutput(e.target.value)} /><button type="submit">保存</button></form><table><thead><tr><th>模型</th><th>输入/1k</th><th>输出/1k</th><th>启用</th></tr></thead><tbody>{items.map((item) => <tr key={item.model}><td>{item.model}</td><td>{item.input_per_1k_micros}</td><td>{item.output_per_1k_micros}</td><td>{item.enabled ? "是" : "否"}</td></tr>)}</tbody></table></section>;
}


interface AuditItem { id: number; action: string; target: string; actor_id: string | null }
function AuditLog({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<AuditItem[]>([]);
  useEffect(() => { void api<{ items: AuditItem[] }>("/api/admin/audit").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); }, []);
  return <section><h2>审计</h2><p className="hint">最近 100 条；哈希链字段在服务端保存。</p><table><thead><tr><th>ID</th><th>动作</th><th>对象</th><th>操作者</th></tr></thead><tbody>{items.map((item) => <tr key={item.id}><td>{item.id}</td><td>{item.action}</td><td>{item.target}</td><td className="mono">{item.actor_id?.slice(0, 8) ?? "system"}</td></tr>)}</tbody></table></section>;
}

interface UsageItem { username: string; requests: number; input_tokens: number; output_tokens: number; cost_micros: number }
function Usage({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<UsageItem[]>([]);
  useEffect(() => { void api<{ items: UsageItem[] }>("/api/admin/usage").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); }, []);
  return <section><h2>用量</h2><table><thead><tr><th>用户</th><th>请求数</th><th>输入 tokens</th><th>输出 tokens</th><th>成本 micros</th></tr></thead><tbody>{items.map((item) => <tr key={item.username}><td>{item.username}</td><td>{item.requests}</td><td>{item.input_tokens}</td><td>{item.output_tokens}</td><td>{item.cost_micros}</td></tr>)}</tbody></table></section>;
}


interface JobItem { job: string; idempotency_key: string; status: string; detail: unknown }
function Jobs({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<JobItem[]>([]);
  const load = () => { void api<{ items: JobItem[] }>("/api/admin/jobs").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const run = async (job: string) => { try { await api(`/api/admin/jobs/${job}/run`, { method: "POST" }); load(); } catch (e) { onError((e as Error).message); } };
  return <section><h2>作业</h2><p className="hint">hold 过期、pending usage 清扫、钱包对账、月发放。</p><div className="row">{([{id:"hold-expire",label:"hold 过期"},{id:"usage-pending",label:"pending 清扫"},{id:"wallet-reconcile",label:"对账"},{id:"wallet-grant",label:"月发放"}] as const).map((action) => <button key={action.id} onClick={() => void run(action.id)}>{action.label}</button>)}</div><table><thead><tr><th>作业</th><th>状态</th><th>幂等键</th></tr></thead><tbody>{items.map((item) => <tr key={item.idempotency_key}><td>{item.job}</td><td>{item.status}</td><td className="mono">{item.idempotency_key.slice(0, 18)}</td></tr>)}</tbody></table></section>;
}


interface SettingItem { key: string; value: unknown; hot: boolean }
function Settings({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<SettingItem[]>([]);
  const [key, setKey] = useState("updates.channel");
  const [value, setValue] = useState('{"channel":"official"}');
  const load = () => { void api<{ items: SettingItem[] }>("/api/admin/settings").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    try { await api(`/api/admin/settings/${key}`, { method: "PUT", body: JSON.stringify({ value: JSON.parse(value) as unknown, hot: true }) }); load(); } catch (e) { onError((e as Error).message); }
  };
  return <section><h2>系统设置</h2><p className="hint">hot=true 立即生效；其余提示重启。值必须是 JSON。</p><form className="row" onSubmit={save}><input value={key} onChange={(e) => setKey(e.target.value)} /><input value={value} onChange={(e) => setValue(e.target.value)} /><button type="submit">保存</button></form><table><thead><tr><th>Key</th><th>Value</th><th>Hot</th></tr></thead><tbody>{items.map((item) => <tr key={item.key}><td>{item.key}</td><td className="mono">{JSON.stringify(item.value)}</td><td>{item.hot ? "是" : "否"}</td></tr>)}</tbody></table></section>;
}
