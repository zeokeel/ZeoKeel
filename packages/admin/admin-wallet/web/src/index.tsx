import { useEffect, useState } from "react";
import { api, type Wallet } from "@zeokeel/admin-ui-kit";

export function Wallets({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<Wallet[]>([]); const [delta, setDelta] = useState("1000000");
  const load = () => { void api<{ items: Wallet[] }>("/api/admin/wallet").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const adjust = async (userId: string) => { try { await api(`/api/admin/wallet/${userId}/adjust`, { method: "POST", body: JSON.stringify({ delta_micros: Number(delta), reason: "admin ui", idempotency_key: `ui-${userId}-${Date.now()}` }) }); load(); } catch (e) { onError((e as Error).message); } };
  return <section><h2>钱包</h2><p className="hint">调账会写 ledger；余额由账本派生。</p><label>调账金额（micros）<input value={delta} onChange={(e) => setDelta(e.target.value)} /></label><table><thead><tr><th>用户</th><th>余额</th><th>操作</th></tr></thead><tbody>{items.map((item) => <tr key={item.user_id}><td>{item.username}</td><td>{item.balance_micros}</td><td><button onClick={() => void adjust(item.user_id)}>调账</button></td></tr>)}</tbody></table></section>;
}


export const components = { "page.wallet": Wallets } as const;
