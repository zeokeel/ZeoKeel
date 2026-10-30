import { useEffect, useState } from "react";
import { api, apiForm, type SkillContent } from "@zeokeel/admin-ui-kit";

export function Skills({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<SkillContent[]>([]);
  const [name, setName] = useState("");
  const [version, setVersion] = useState("1.0.0");
  const [file, setFile] = useState<File | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const load = () => { void api<{ items: SkillContent[] }>("/api/admin/skills").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const fail = (e: unknown) => { onError(e instanceof Error ? e.message : "request failed"); };
  const create = async (event: React.FormEvent) => {
    event.preventDefault();
    try { await api("/api/admin/skills", { method: "POST", body: JSON.stringify({ name }) }); setName(""); load(); } catch (e) { fail(e); }
  };
  const upload = async (id: string) => {
    if (file === null) { onError("请选择 zip"); return; }
    const form = new FormData();
    form.set("version", version);
    form.set("archive", file);
    setBusy(true);
    try { await apiForm(`/api/admin/skills/${id}/versions`, form); setFile(null); load(); } catch (e) { fail(e); } finally { setBusy(false); }
  };
  const act = async (id: string, action: string) => {
    try { await api(`/api/admin/skills/${id}/${action}`, { method: "POST" }); load(); } catch (e) { fail(e); }
  };
  const remove = async (id: string) => {
    try { await api(`/api/admin/skills/${id}`, { method: "DELETE" }); setSelected(null); load(); } catch (e) { fail(e); }
  };
  return (
    <section>
      <h2>技能</h2>
      <form className="row" onSubmit={create}>
        <input aria-label="技能名" placeholder="技能名" value={name} onChange={(e) => setName(e.target.value)} />
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
                {item.status === "pending" ? <button type="button" onClick={() => void act(item.id, "approve")}>审批</button> : null}
                {item.status === "pending" ? <button type="button" onClick={() => void act(item.id, "reject")}>拒绝</button> : null}
                {item.status === "approved" ? <button type="button" onClick={() => void act(item.id, "delist")}>下架</button> : null}
                <button type="button" onClick={() => void remove(item.id)}>删除</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {selected !== null ? (
        <div className="row" aria-label="技能版本上传">
          <input aria-label="版本" value={version} onChange={(e) => setVersion(e.target.value)} />
          <input aria-label="技能 zip" type="file" accept=".zip" onChange={(e) => setFile(e.target.files?.[0] ?? null)} />
          <button type="button" disabled={busy} onClick={() => void upload(selected)}>{busy ? "上传中…" : "上传版本"}</button>
          <button type="button" onClick={() => void api(`/api/admin/skills/${selected}/grants`, { method: "PUT", body: JSON.stringify({ grants: [{ grantee: "all", can_install: true }] }) }).then(load).catch(fail)}>全员可安装</button>
          <button type="button" onClick={() => void api(`/api/admin/skills/${selected}/force-update`, { method: "POST", body: JSON.stringify({ force: true, min_client_version: "0.1.0" }) }).then(load).catch(fail)}>强制更新</button>
        </div>
      ) : null}
    </section>
  );
}

export const components = { "page.skills": Skills } as const;
