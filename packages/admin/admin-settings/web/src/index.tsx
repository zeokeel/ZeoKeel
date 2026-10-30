import { useCallback, useEffect, useState } from "react";
import { api, messageOf, type SettingItem } from "@zeokeel/admin-ui-kit";

export function Settings({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<SettingItem[]>([]);
  const [key, setKey] = useState("updates.channel");
  const [value, setValue] = useState('{"channel":"official"}');
  const [hot, setHot] = useState(true);
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: SettingItem[] }>("/api/admin/settings").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
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
      <h2>系统设置</h2>
      <p className="hint">hot=true 立即生效；其余提示重启。值必须是 JSON。</p>
      <form className="row" onSubmit={save}>
        <input aria-label="设置 Key" value={key} onChange={(event) => { setKey(event.target.value); }} />
        <input aria-label="设置 Value" value={value} onChange={(event) => { setValue(event.target.value); }} />
        <label><input type="checkbox" checked={hot} onChange={(event) => { setHot(event.target.checked); }} /> hot</label>
        <button type="submit">保存</button>
      </form>
      <table>
        <thead><tr><th>Key</th><th>Value</th><th>Hot</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.key}><td>{item.key}</td><td className="mono">{JSON.stringify(item.value)}</td><td>{item.hot ? "是" : "否"}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.settings": Settings } as const;
