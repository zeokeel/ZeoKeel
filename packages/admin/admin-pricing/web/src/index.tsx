import { useCallback, useEffect, useState } from "react";
import { api, messageOf, type Price } from "@zeokeel/admin-ui-kit";

export function Pricing({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<Price[]>([]);
  const [model, setModel] = useState("deepseek-chat");
  const [input, setInput] = useState("1000");
  const [output, setOutput] = useState("2000");
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void api<{ items: Price[] }>("/api/admin/pricing").then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
  useEffect(() => { load(); }, [load]);
  const save = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    void (async () => {
      await api(`/api/admin/pricing/${model}`, {
        method: "PUT",
        body: JSON.stringify({ input_per_1k_micros: Number(input), output_per_1k_micros: Number(output) }),
      });
      load();
    })().catch(fail);
  };
  return (
    <section>
      <h2>模型价格</h2>
      <form className="row" onSubmit={save}>
        <input aria-label="模型" value={model} onChange={(event) => { setModel(event.target.value); }} />
        <input aria-label="输入价格" value={input} onChange={(event) => { setInput(event.target.value); }} />
        <input aria-label="输出价格" value={output} onChange={(event) => { setOutput(event.target.value); }} />
        <button type="submit">保存</button>
      </form>
      <table>
        <thead><tr><th>模型</th><th>输入/1k</th><th>输出/1k</th><th>启用</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.model}><td>{item.model}</td><td>{String(item.input_per_1k_micros)}</td><td>{String(item.output_per_1k_micros)}</td><td>{item.enabled ? "是" : "否"}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.pricing": Pricing } as const;
