import { ArrayInput, SimpleFormIterator, TextInput } from "@/components/admin";

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

export const HeadersInput = () => (
  <ArrayInput source="headers" format={mapToPairs} parse={pairsToMap} label="Headers">
    <SimpleFormIterator inline>
      <TextInput source="name" label="Header" helperText={false} />
      <TextInput source="value" label="Value" helperText={false} />
    </SimpleFormIterator>
  </ArrayInput>
);
