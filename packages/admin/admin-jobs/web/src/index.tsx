import { useEffect, useState } from "react";
import { api, type JobItem } from "@zeokeel/admin-ui-kit";

export function Jobs({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<JobItem[]>([]);
  const load = () => { void api<{ items: JobItem[] }>("/api/admin/jobs").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const run = async (job: string) => { try { await api(`/api/admin/jobs/${job}/run`, { method: "POST" }); load(); } catch (e) { onError((e as Error).message); } };
  return <section><h2>作业</h2><p className="hint">hold 过期、pending usage 清扫、钱包对账、月发放。</p><div className="row">{([{id:"hold-expire",label:"hold 过期"},{id:"usage-pending",label:"pending 清扫"},{id:"wallet-reconcile",label:"对账"},{id:"wallet-grant",label:"月发放"}] as const).map((action) => <button key={action.id} onClick={() => void run(action.id)}>{action.label}</button>)}</div><table><thead><tr><th>作业</th><th>状态</th><th>幂等键</th></tr></thead><tbody>{items.map((item) => <tr key={item.idempotency_key}><td>{item.job}</td><td>{item.status}</td><td className="mono">{item.idempotency_key.slice(0, 18)}</td></tr>)}</tbody></table></section>;
}

