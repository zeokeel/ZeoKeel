import { useCallback, useEffect, useState } from "react";
import { api, messageOf, type JobItem } from "@zeokeel/admin-ui-kit";

const ACTIONS = [
  { id: "hold-expire", label: "hold 过期" },
  { id: "usage-pending", label: "pending 清扫" },
  { id: "wallet-reconcile", label: "对账" },
  { id: "wallet-grant", label: "月发放" },
] as const;

export function Jobs({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<JobItem[]>([]);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: JobItem[] }>("/api/admin/jobs").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const run = (job: string): void => { void api(`/api/admin/jobs/${job}/run`, { method: "POST" }).then(load).catch(fail); };
  return (
    <section>
      <h2>作业</h2>
      <p className="hint">hold 过期、pending usage 清扫、钱包对账、月发放。</p>
      <div className="row">{ACTIONS.map((action) => <button key={action.id} type="button" onClick={() => { run(action.id); }}>{action.label}</button>)}</div>
      <table>
        <thead><tr><th>作业</th><th>状态</th><th>幂等键</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.idempotency_key}><td>{item.job}</td><td>{item.status}</td><td className="mono">{item.idempotency_key.slice(0, 18)}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.jobs": Jobs } as const;
