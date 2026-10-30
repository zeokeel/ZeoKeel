import { useCallback, useEffect, useState } from "react";
import { adminT, api, assertValid, messageOf, validateAdjust, type Wallet } from "@zeokeel/admin-ui-kit";

export function Wallets({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<Wallet[]>([]);
  const [delta, setDelta] = useState("1000000");
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: Wallet[] }>("/api/admin/wallet").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const adjust = (userId: string): void => {
    void (async () => {
      // Event-handler timestamp for idempotency; not part of render.
      const stamp = Date.now();
      const payload = { delta_micros: Number(delta), reason: "admin ui", idempotency_key: `ui-${userId}-${String(stamp)}` };
      assertValid(validateAdjust, payload, "ADJUST");
      await api(`/api/admin/wallet/${userId}/adjust`, { method: "POST", body: JSON.stringify(payload) });
      load();
    })().catch(fail);
  };
  return (
    <section>
      <h2>{adminT("wallet.title")}</h2>
      <p className="hint">{adminT("wallet.hint")}</p>
      <label>{adminT("wallet.amount")}<input value={delta} onChange={(event) => { setDelta(event.target.value); }} /></label>
      <table>
        <thead><tr><th>{adminT("common.user")}</th><th>{adminT("wallet.colBalance")}</th><th>{adminT("common.actions")}</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.user_id}><td>{item.username}</td><td>{String(item.balance_micros)}</td><td><button type="button" onClick={() => { adjust(item.user_id); }}>{adminT("common.adjust")}</button></td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.wallet": Wallets } as const;
