import { useEffect, useState } from "react";
import { api, type SettingItem } from "@zeokeel/admin-ui-kit";

export function Settings({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<SettingItem[]>([]);
  const [key, setKey] = useState("updates.channel");
  const [value, setValue] = useState('{"channel":"official"}');
  const load = () => { void api<{ items: SettingItem[] }>("/api/admin/settings").then((r) => setItems(r.items)).catch((e: Error) => onError(e.message)); };
  useEffect(load, []);
  const save = async (event: React.FormEvent) => {
    event.preventDefault();
    try { await api(`/api/admin/settings/${key}`, { method: "PUT", body: JSON.stringify({ value: JSON.parse(value) as unknown, hot: true }) }); load(); } catch (e) { onError((e as Error).message); }
  };
  return <section><h2>系统设置</h2><p className="hint">hot=true 立即生效；其余提示重启。值必须是 JSON。</p><form className="row" onSubmit={save}><input value={key} onChange={(e) => setKey(e.target.value)} /><input value={value} onChange={(e) => setValue(e.target.value)} /><button type="submit">保存</button></form><table><thead><tr><th>Key</th><th>Value</th><th>Hot</th></tr></thead><tbody>{items.map((item) => <tr key={item.key}><td>{item.key}</td><td className="mono">{JSON.stringify(item.value)}</td><td>{item.hot ? "是" : "否"}</td></tr>)}</tbody></table></section>;
}


export const components = { "page.settings": Settings } as const;
