import { useCallback, useEffect, useState } from "react";
import { adminT, api, apiJson, decodeAdminSettings, messageOf, type AdminSettingItem } from "@zeokeel/admin-ui-kit";

export function Settings({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<AdminSettingItem[]>([]);
  const [key, setKey] = useState("updates.channel");
  const [value, setValue] = useState('{"channel":"official"}');
  const [hot, setHot] = useState(true);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void apiJson("/api/admin/settings", decodeAdminSettings).then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const save = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => {
      const parsed: unknown = JSON.parse(value);
      await api(`/api/admin/settings/${key}`, { method: "PUT", body: JSON.stringify({ value: parsed, hot }) });
      load();
    })().catch(fail);
  };
  return (
    <section>
      <h2>{adminT("settings.title")}</h2>
      <p className="hint">{adminT("settings.hint")}</p>
      <form className="row" onSubmit={save}>
        <input aria-label={adminT("settings.ariaKey")} value={key} onChange={(event) => { setKey(event.target.value); }} />
        <input aria-label={adminT("settings.ariaValue")} value={value} onChange={(event) => { setValue(event.target.value); }} />
        <label><input type="checkbox" checked={hot} onChange={(event) => { setHot(event.target.checked); }} /> {adminT("settings.hot")}</label>
        <button type="submit">{adminT("common.save")}</button>
      </form>
      <table>
        <thead><tr><th>{adminT("common.key")}</th><th>{adminT("common.value")}</th><th>{adminT("common.hot")}</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.key}><td>{item.key}</td><td className="mono">{JSON.stringify(item.value)}</td><td>{item.hot ? adminT("common.yes") : adminT("common.no")}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.settings": Settings } as const;
