import { useCallback, useEffect, useState } from "react";
import { adminT, api, apiForm, apiJson, decodeAdminSkills, messageOf, type AdminSkillItem } from "@zeokeel/admin-ui-kit";

export function Skills({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<AdminSkillItem[]>([]);
  const [name, setName] = useState("");
  const [version, setVersion] = useState("1.0.0");
  const [file, setFile] = useState<File | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void apiJson("/api/admin/skills", decodeAdminSkills).then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const create = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => { await api("/api/admin/skills", { method: "POST", body: JSON.stringify({ name }) }); setName(""); load(); })().catch(fail);
  };
  const upload = (id: string): void => {
    if (file === null) { onError(adminT("skills.selectZip")); return; }
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
      <h2>{adminT("skills.title")}</h2>
      <form className="row" onSubmit={create}>
        <input aria-label={adminT("skills.ariaName")} placeholder={adminT("skills.ariaName")} value={name} onChange={(event) => { setName(event.target.value); }} />
        <button type="submit">{adminT("skills.create")}</button>
      </form>
      <p className="hint">{adminT("skills.hint")}</p>
      <table>
        <thead><tr><th>{adminT("common.name")}</th><th>{adminT("common.version")}</th><th>{adminT("common.status")}</th><th>{adminT("common.visibility")}</th><th>{adminT("common.force")}</th><th>{adminT("common.actions")}</th></tr></thead>
        <tbody>
          {items.length === 0 ? <tr><td colSpan={6}>{adminT("skills.empty")}</td></tr> : null}
          {items.map((item) => (
            <tr key={item.id}>
              <td>{item.name}</td>
              <td>{item.version === "" ? adminT("common.none") : item.version}</td>
              <td>{item.status}</td>
              <td>{item.visibility}</td>
              <td>{item.force_update ? `${adminT("common.yes")} (${item.min_client_version})` : adminT("common.no")}</td>
              <td>
                <button type="button" onClick={() => { setSelected(item.id); }}>{adminT("common.manage")}</button>
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "approve"); }}>{adminT("common.approve")}</button> : null}
                {item.status === "pending" ? <button type="button" onClick={() => { act(item.id, "reject"); }}>{adminT("common.reject")}</button> : null}
                {item.status === "approved" ? <button type="button" onClick={() => { act(item.id, "delist"); }}>{adminT("common.delist")}</button> : null}
                <button type="button" onClick={() => { remove(item.id); }}>{adminT("common.delete")}</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {selected !== null ? (
        <div className="row" aria-label={adminT("skills.ariaUpload")}>
          <input aria-label={adminT("skills.ariaVersion")} value={version} onChange={(event) => { setVersion(event.target.value); }} />
          <input aria-label={adminT("skills.ariaZip")} type="file" accept=".zip" onChange={(event) => { setFile(event.target.files?.[0] ?? null); }} />
          <button type="button" disabled={busy} onClick={() => { upload(selected); }}>{busy ? adminT("skills.uploading") : adminT("skills.uploadVersion")}</button>
          <button type="button" onClick={() => { void api(`/api/admin/skills/${selected}/grants`, { method: "PUT", body: JSON.stringify({ grants: [{ grantee: "all", can_install: true }] }) }).then(load).catch(fail); }}>{adminT("common.grantAllInstall")}</button>
          <button type="button" onClick={() => { void api(`/api/admin/skills/${selected}/force-update`, { method: "POST", body: JSON.stringify({ force: true, min_client_version: "0.1.0" }) }).then(load).catch(fail); }}>{adminT("common.forceUpdate")}</button>
        </div>
      ) : null}
    </section>
  );
}

export const components = { "page.skills": Skills } as const;
