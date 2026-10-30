import { useEffect, useState } from "react";
import { api, type AuditItem } from "@zeokeel/admin-ui-kit";

export function AuditLog({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<AuditItem[]>([]);
  useEffect(() => { void api<{ items: AuditItem[] }>("/api/admin/audit").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); }, []);
  return <section><h2>审计</h2><p className="hint">最近 100 条；哈希链字段在服务端保存。</p><table><thead><tr><th>ID</th><th>动作</th><th>对象</th><th>操作者</th></tr></thead><tbody>{items.map((item) => <tr key={item.id}><td>{item.id}</td><td>{item.action}</td><td>{item.target}</td><td className="mono">{item.actor_id?.slice(0, 8) ?? "system"}</td></tr>)}</tbody></table></section>;
}


export const components = { "page.audit": AuditLog } as const;
