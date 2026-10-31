import { useCallback, useEffect, useState } from "react";
import { adminT, apiJson, decodeAdminUsage, messageOf, type AdminUsageItem } from "@zeokeel/admin-ui-kit";

export function Usage({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<AdminUsageItem[]>([]);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void apiJson("/api/admin/usage", decodeAdminUsage).then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  return (
    <section>
      <h2>{adminT("usage.title")}</h2>
      <table>
        <thead><tr><th>{adminT("common.user")}</th><th>{adminT("usage.colRequests")}</th><th>{adminT("usage.colInputTokens")}</th><th>{adminT("usage.colOutputTokens")}</th><th>{adminT("usage.colCost")}</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.username}><td>{item.username}</td><td>{String(item.requests)}</td><td>{String(item.input_tokens)}</td><td>{String(item.output_tokens)}</td><td>{String(item.cost_micros)}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.usage": Usage } as const;
