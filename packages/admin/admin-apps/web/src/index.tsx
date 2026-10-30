import { useCallback, useEffect, useState } from "react";
import { api, messageOf, type AppAdmin } from "@zeokeel/admin-ui-kit";

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
      <h2>应用</h2>
      <p className="hint">WASM 服务端执行，客户端不执行应用；下架后新打开被拒绝。</p>
      <table>
        <thead><tr><th>名称</th><th>版本</th><th>状态</th><th>wasm sha</th><th>操作</th></tr></thead>
        <tbody>
          {items.length === 0 ? <tr><td colSpan={5}>暂无应用</td></tr> : null}
          {items.map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td><td>{item.version === "" ? "—" : item.version}</td><td>{item.status}</td><td className="mono">{item.wasm_sha256.slice(0, 12)}</td>
              <td>
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "approve"); }}>审批</button> : null}
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "reject"); }}>拒绝</button> : null}
                {item.status === "approved" ? <button type="button" onClick={() => { act(item.id, "delist"); }}>下架</button> : null}
                <button type="button" onClick={() => { grantAll(item.id); }}>全员可用</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

export const components = { "page.apps": Apps } as const;
