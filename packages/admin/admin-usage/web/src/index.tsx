import { useEffect, useState } from "react";
import { api, type UsageItem } from "@zeokeel/admin-ui-kit";

export function Usage({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<UsageItem[]>([]);
  useEffect(() => { void api<{ items: UsageItem[] }>("/api/admin/usage").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); }, []);
  return <section><h2>用量</h2><table><thead><tr><th>用户</th><th>请求数</th><th>输入 tokens</th><th>输出 tokens</th><th>成本 micros</th></tr></thead><tbody>{items.map((item) => <tr key={item.username}><td>{item.username}</td><td>{item.requests}</td><td>{item.input_tokens}</td><td>{item.output_tokens}</td><td>{item.cost_micros}</td></tr>)}</tbody></table></section>;
}

