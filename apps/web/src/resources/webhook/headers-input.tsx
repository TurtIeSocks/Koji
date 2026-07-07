import { useState } from "react";
import { useController } from "react-hook-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function mapToPairs(
  map: Record<string, string> | null | undefined,
): { name: string; value: string }[] {
  if (!map) return [];
  return Object.entries(map).map(([name, value]) => ({ name, value: String(value) }));
}

export function pairsToMap(
  pairs: { name?: string; value?: string }[] | null | undefined,
): Record<string, string> {
  const out: Record<string, string> = {};
  for (const p of pairs ?? []) {
    const name = (p?.name ?? "").trim();
    if (!name) continue;
    out[name] = p?.value ?? "";
  }
  return out;
}

type HeaderRow = { name: string; value: string };

// ArrayInput's SimpleFormIterator drives react-hook-form's useFieldArray
// directly and never reads format/parse (those props only apply to leaf
// inputs wired via useInput). Since `headers` is a Record<string, string>
// on the wire — not an array — we bind it ourselves via useController and
// keep the editable rows in local state so in-progress edits (blank names,
// duplicate keys) don't get silently dropped by re-deriving from the map.
export const HeadersInput = () => {
  const { field } = useController<{ headers: Record<string, string> }, "headers">({
    name: "headers",
    defaultValue: {},
  });
  const [rows, setRows] = useState<HeaderRow[]>(() => mapToPairs(field.value));

  const sync = (next: HeaderRow[]) => {
    setRows(next);
    field.onChange(pairsToMap(next));
  };

  const updateRow = (index: number, patch: Partial<HeaderRow>) => {
    sync(rows.map((row, i) => (i === index ? { ...row, ...patch } : row)));
  };

  const removeRow = (index: number) => {
    sync(rows.filter((_, i) => i !== index));
  };

  const addRow = () => {
    sync([...rows, { name: "", value: "" }]);
  };

  return (
    <div className="flex flex-col gap-2">
      <span className="text-sm font-medium">Headers</span>
      {rows.map((row, index) => (
        <div key={index} className="flex items-center gap-2">
          <Input
            aria-label="Header"
            placeholder="Header"
            value={row.name}
            onChange={(e) => updateRow(index, { name: e.target.value })}
          />
          <Input
            aria-label="Value"
            placeholder="Value"
            value={row.value}
            onChange={(e) => updateRow(index, { value: e.target.value })}
          />
          <Button type="button" variant="ghost" size="icon-sm" onClick={() => removeRow(index)}>
            &times;
          </Button>
        </div>
      ))}
      <Button type="button" variant="outline" size="sm" onClick={addRow} className="self-start">
        Add header
      </Button>
    </div>
  );
};
