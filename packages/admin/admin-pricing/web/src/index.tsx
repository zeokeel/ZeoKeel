import { useCallback, useEffect, useState } from "react";
import { adminT, api, apiJson, decodeAdminPrices, messageOf, type ModelPrice } from "@zeokeel/admin-ui-kit";

export function Pricing({ onError }: { onError: (message: string) => void }) {
  const [items, setItems] = useState<ModelPrice[]>([]);
  const [model, setModel] = useState("deepseek-chat");
  const [input, setInput] = useState("1000");
  const [output, setOutput] = useState("2000");
  const fail = useCallback((error: unknown) => { onError(messageOf(error)); }, [onError]);
  const load = useCallback(() => { void apiJson("/api/admin/pricing", decodeAdminPrices).then((r) => { setItems(r.items); }).catch(fail); }, [fail]);
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
      <h2>{adminT("pricing.title")}</h2>
      <form className="row" onSubmit={save}>
        <input aria-label={adminT("pricing.ariaModel")} value={model} onChange={(event) => { setModel(event.target.value); }} />
        <input aria-label={adminT("pricing.ariaInput")} value={input} onChange={(event) => { setInput(event.target.value); }} />
        <input aria-label={adminT("pricing.ariaOutput")} value={output} onChange={(event) => { setOutput(event.target.value); }} />
        <button type="submit">{adminT("common.save")}</button>
      </form>
      <table>
        <thead><tr><th>{adminT("pricing.colModel")}</th><th>{adminT("pricing.colInput")}</th><th>{adminT("pricing.colOutput")}</th><th>{adminT("pricing.colEnabled")}</th></tr></thead>
        <tbody>{items.map((item) => <tr key={item.model}><td>{item.model}</td><td>{String(item.input_per_1k_micros)}</td><td>{String(item.output_per_1k_micros)}</td><td>{item.enabled ? adminT("common.yes") : adminT("common.no")}</td></tr>)}</tbody>
      </table>
    </section>
  );
}

export const components = { "page.pricing": Pricing } as const;
