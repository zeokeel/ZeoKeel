import { useCallback, useEffect, useState } from "react";
import { adminT, api, messageOf, type AuditItem } from "@zeokeel/admin-ui-kit";

export function AuditLog({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<AuditItem[]>([]);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: AuditItem[] }>("/api/admin/audit").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  return (
    <section>
      <h2>{adminT("audit.title")}</h2>
      <p className="hint">{adminT("audit.hint")}</p>
      <table>
        <thead><tr><th>ID</th><th>{adminT("audit.colAction")}</th><th>{adminT("audit.colTarget")}</th><th>{adminT("audit.colActor")}</th></tr></thead>
        <tbody>
          {items.map((item) => (
            <tr key={item.id}><td>{String(item.id)}</td><td>{item.action}</td><td>{item.target}</td><td className="mono">{item.actor_id?.slice(0, 8) ?? "system"}</td></tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

export const components = { "page.audit": AuditLog } as const;
