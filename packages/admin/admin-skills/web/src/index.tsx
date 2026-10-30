import { useCallback, useEffect, useState } from "react";
import { api, apiForm, messageOf, type SkillContent } from "@zeokeel/admin-ui-kit";

export function Skills({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<SkillContent[]>([]);
  const [name, setName] = useState("");
  const [version, setVersion] = useState("1.0.0");
  const [file, setFile] = useState<File | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: SkillContent[] }>("/api/admin/skills").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const create = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => { await api("/api/admin/skills", { method: "POST", body: JSON.stringify({ name }) }); setName(""); load(); })().catch(fail);
  };
  const upload = (id: string): void => {
    if (file === null) { onError("请选择 zip"); return; }
    const form = new FormData();
    form.set("version", version);
    form.set("archive", file);
    setBusy(true);
    void (async () => { await apiForm(`/api/admin/skills/${id}/versions`, form); setFile(null); load(); })()
      .catch(fail)
      .finally(() => { setBusy(false); });
  };
  const act = (id: string, action: string): void => { void api(`/api/admin/skills/${id}/${action}`, { method: "POST" }).then(load).catch(fail); };
  const remove = (id: string): void => { void api(`/api/admin/skills/${id}`, { method: "DELETE" }).then(() => { setSelected(null); load(); }).catch(fail); };
  return (
    <section>
      <h2>技能</h2>
      <form className="row" onSubmit={create}>
        <input aria-label="技能名" placeholder="技能名" value={name} onChange={(event) => { setName(event.target.value); }} />
        <button type="submit">创建技能</button>
      </form>
      <p className="hint">上传 zip ≤16MB，必须包含 SKILL.md；服务端只校验归档，不解压。</p>
      <table>
        <thead><tr><th>名称</th><th>版本</th><th>状态</th><th>可见性</th><th>Force</th><th>操作</th></tr></thead>
        <tbody>
          {items.length === 0 ? <tr><td colSpan={6}>暂无技能</td></tr> : null}
          {items.map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td>
              <td>{item.version === "" ? "—" : item.version}</td>
              <td>{item.status}</td>
              <td>{item.visibility}</td>
              <td>{item.force_update ? `是 (${item.min_client_version})` : "否"}</td>
              <td>
                <button type="button" onClick={() => { setSelected(item.id); }}>管理</button>
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "approve"); }}>审批</button> : null}
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "reject"); }}>拒绝</button> : null}
                {item.status === "approved" ? <button type="button" onClick={() => { act(item.id, "delist"); }}>下架</button> : null}
                <button type="button" onClick={() => { remove(item.id); }}>删除</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {selected !== null ? (
        <div className="row" aria-label="技能版本上传">
          <input aria-label="版本" value={version} onChange={(event) => { setVersion(event.target.value); }} />
          <input aria-label="技能 zip" type="file" accept=".zip" onChange={(event) => { setFile(event.target.files?.[0] ?? null); }} />
          <button type="button" disabled={busy} onClick={() => { upload(selected); }}>{busy ? "上传中…" : "上传版本"}</button>
          <button type="button" onClick={() => { void api(`/api/admin/skills/${selected}/grants`, { method: "PUT", body: JSON.stringify({ grants: [{ grantee: "all", can_install: true }] }) }).then(load).catch(fail); }}>全员可安装</button>
          <button type="button" onClick={() => { void api(`/api/admin/skills/${selected}/force-update`, { method: "POST", body: JSON.stringify({ force: true, min_client_version: "0.1.0" }) }).then(load).catch(fail); }}>强制更新</button>
        </div>
      ) : null}
    </section>
  );
}

export const components = { "page.skills": Skills } as const;
