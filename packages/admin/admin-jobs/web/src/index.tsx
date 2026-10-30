import { useCallback, useEffect, useState } from "react";
import { adminT, api, messageOf, type JobItem } from "@zeokeel/admin-ui-kit";

const ACTIONS = [
  { id: "hold-expire", label: "jobs.holdExpire" },
  { id: "usage-pending", label: "jobs.usagePending" },
  { id: "wallet-reconcile", label: "jobs.reconcile" },
  { id: "wallet-grant", label: "jobs.monthly" },
] as const;

export function Jobs({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<JobItem[]>([]);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: JobItem[] }>("/api/admin/jobs").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const run = (job: string): void => { void api(`/api/admin/jobs/${job}/run`, { method: "POST" }).then(load).catch(fail); };
  return (
    <section>
      <h2>{adminT("jobs.title")}</h2>
      <p className="hint">{adminT("jobs.hint")}</p>
      <div className="row">{ACTIONS.map((action) => <button key={action.id} type="button" onClick={() => { run(action.id); }}>{adminT(action.label)}</button>)}</div>
      <table>
        <thead><tr><th>{adminT("jobs.colJob")}</th><th>{adminT("common.status")}</th><th>{adminT("jobs.colIdempotency")}</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.idempotency_key}><td>{item.job}</td><td>{item.status}</td><td className="mono">{item.idempotency_key.slice(0, 18)}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.jobs": Jobs } as const;
