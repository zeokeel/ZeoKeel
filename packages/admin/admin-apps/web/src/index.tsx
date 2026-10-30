import { useCallback, useEffect, useState } from "react";
import { adminT, api, messageOf, type AppAdmin } from "@zeokeel/admin-ui-kit";

export function Apps({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<AppAdmin[]>([]);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: AppAdmin[] }>("/api/admin/apps").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const act = (id: string, action: string): void => { void api(`/api/admin/apps/${id}/${action}`, { method: "POST" }).then(load).catch(fail); };
  const grantAll = (id: string): void => {
    void api(`/api/admin/apps/${id}/grants`, { method: "PUT", body: JSON.stringify({ grants: [{ grantee: "all", can_open: true }] }) }).then(load).catch(fail);
  };
  return (
    <section>
      <h2>{adminT("apps.title")}</h2>
      <p className="hint">{adminT("apps.hint")}</p>
      <table>
        <thead><tr><th>{adminT("common.name")}</th><th>{adminT("common.version")}</th><th>{adminT("common.status")}</th><th>{adminT("apps.colWasm")}</th><th>{adminT("common.actions")}</th></tr></thead>
        <tbody>
          {items.length === 0 ? <tr><td colSpan={5}>{adminT("apps.empty")}</td></tr> : null}
          {items.map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td><td>{item.version === "" ? adminT("common.none") : item.version}</td><td>{item.status}</td><td className="mono">{item.wasm_sha256.slice(0, 12)}</td>
              <td>
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "approve"); }}>{adminT("common.approve")}</button> : null}
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "reject"); }}>{adminT("common.reject")}</button> : null}
                {item.status === "approved" ? <button type="button" onClick={() => { act(item.id, "delist"); }}>{adminT("common.delist")}</button> : null}
                <button type="button" onClick={() => { grantAll(item.id); }}>{adminT("common.grantAll")}</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

export const components = { "page.apps": Apps } as const;
