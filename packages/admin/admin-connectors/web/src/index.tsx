import { useCallback, useEffect, useState } from "react";
import { api, isRecord, messageOf, type ConnectorAdmin } from "@zeokeel/admin-ui-kit";

export function Connectors({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<ConnectorAdmin[]>([]);
  const [name, setName] = useState("");
  const [transport, setTransport] = useState("http");
  const [selected, setSelected] = useState<string | null>(null);
  const [clientId, setClientId] = useState("");
  const [redirect, setRedirect] = useState("http://127.0.0.1/callback");
  const [authorizeUrl, setAuthorizeUrl] = useState("https://auth.example/authorize");
  const [tokenUrl, setTokenUrl] = useState("https://auth.example/token");
  const [createConfig, setCreateConfig] = useState("{}");
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: ConnectorAdmin[] }>("/api/admin/connectors").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const create = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => {
      const parsed: unknown = JSON.parse(createConfig);
      if (!isRecord(parsed)) {
        onError("config 必须是 JSON 对象");
        return;
      }
      const config: Record<string, unknown> = { ...parsed, authorize_url: authorizeUrl, token_url: tokenUrl };
      await api("/api/admin/connectors", { method: "POST", body: JSON.stringify({ name, transport, config }) });
      setName("");
      load();
    })().catch(fail);
  };
  const act = (id: string, path: string, method: "POST" | "PUT", body: unknown): void => {
    void api(`/api/admin/connectors/${id}/${path}`, { method, body: JSON.stringify(body) }).then(load).catch(fail);
  };
  const remove = (id: string): void => { void api(`/api/admin/connectors/${id}`, { method: "DELETE" }).then(load).catch(fail); };
  return (
    <section>
      <h2>连接器</h2>
      <form className="row" onSubmit={create}>
        <input aria-label="连接器名" placeholder="连接器名" value={name} onChange={(event) => { setName(event.target.value); }} />
        <select aria-label="transport" value={transport} onChange={(event) => { setTransport(event.target.value); }}><option value="http">http</option><option value="sse">sse</option><option value="stdio">stdio</option></select>
        <input aria-label="connector config" value={createConfig} onChange={(event) => { setCreateConfig(event.target.value); }} />
        <button type="submit">创建连接器</button>
      </form>
      <p className="hint">M3 仅无静态 secret 配置；OAuth 公共客户端 + PKCE，令牌只在客户端本地。</p>
      <table>
        <thead><tr><th>名称</th><th>transport</th><th>grants</th><th>OAuth</th><th>操作</th></tr></thead>
        <tbody>
          {items.length === 0 ? <tr><td colSpan={5}>暂无连接器</td></tr> : null}
          {items.map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td>
              <td>{item.transport}</td>
              <td>{item.grants.map((grant) => `${grant.grantee}:${grant.can_enable ? "on" : "off"}`).join(", ") || "—"}</td>
              <td>{item.oauth_client_id ?? "—"}</td>
              <td>
                <button type="button" onClick={() => { setSelected(item.id); }}>管理</button>
                <button type="button" onClick={() => { act(item.id, "grants", "PUT", { grants: [{ grantee: "all", can_enable: true }] }); }}>全员可用</button>
                <button type="button" onClick={() => { remove(item.id); }}>删除</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {selected !== null ? (
        <div className="row" aria-label="连接器配置">
          <input aria-label="OAuth client_id" placeholder="client_id" value={clientId} onChange={(event) => { setClientId(event.target.value); }} />
          <input aria-label="OAuth redirect_uri" value={redirect} onChange={(event) => { setRedirect(event.target.value); }} />
          <button type="button" onClick={() => { void api(`/api/admin/connectors/${selected}/oauth`, { method: "POST", body: JSON.stringify({ client_id: clientId, scopes: ["mcp"], redirect_uri: redirect }) }).then(load).catch(fail); }}>保存 OAuth</button>
          <input aria-label="authorize_url" value={authorizeUrl} onChange={(event) => { setAuthorizeUrl(event.target.value); }} />
          <input aria-label="token_url" value={tokenUrl} onChange={(event) => { setTokenUrl(event.target.value); }} />
          <button type="button" onClick={() => { act(selected, "force-update", { force: true, min_client_version: "0.1.0" }); }}>强制更新</button>
        </div>
      ) : null}
    </section>
  );
}

export const components = { "page.connectors": Connectors } as const;
