import { useCallback, useEffect, useState } from "react";
import { api, messageOf, type UsageItem } from "@zeokeel/admin-ui-kit";

export function Usage({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<UsageItem[]>([]);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: UsageItem[] }>("/api/admin/usage").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  return (
    <section>
      <h2>用量</h2>
      <table>
        <thead><tr><th>用户</th><th>请求数</th><th>输入 tokens</th><th>输出 tokens</th><th>成本 micros</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.username}><td>{item.username}</td><td>{String(item.requests)}</td><td>{String(item.input_tokens)}</td><td>{String(item.output_tokens)}</td><td>{String(item.cost_micros)}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.usage": Usage } as const;
