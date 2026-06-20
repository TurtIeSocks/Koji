import { useState, useEffect } from "react";
import { useWatch, useFormContext } from "react-hook-form";
import { RecordContextProvider } from "shadmin-core";
import { FeatureCollectionField } from "@/components/leaflet";
import { cn } from "@/lib/utils";

interface Feature {
  type: "Feature";
  geometry: unknown;
  properties: Record<string, unknown> | null;
}

/** Resolve the name for feature `i` given a name-property key and template. */
function resolveName(f: Feature, i: number, nameProp: string, template: string): string {
  const nameVal = nameProp ? String(f.properties?.[nameProp] ?? "") : "";
  return template.replaceAll("{name}", nameVal).replaceAll("{index}", String(i));
}

function MapNameStep() {
  const features = (useWatch({ name: "features" }) as Feature[]) ?? [];
  const { setValue } = useFormContext();

  const propKeys = Array.from(
    new Set(features.flatMap((f) => Object.keys(f.properties ?? {}))),
  );

  const [nameProp, setNameProp] = useState("");
  const [template, setTemplate] = useState("{name}");

  // Persist naming choice to sibling form fields (no loop — not watching these fields).
  useEffect(() => {
    setValue("_name_prop", nameProp, { shouldDirty: true });
    setValue("_name_template", template, { shouldDirty: true });
  }, [nameProp, template, setValue]);

  // Compute resolved names for warnings.
  const resolvedNames = features.map((f, i) => resolveName(f, i, nameProp, template));
  const emptyCount = resolvedNames.filter((n) => !n.trim()).length;
  const seen = new Map<string, number>();
  for (const n of resolvedNames) seen.set(n, (seen.get(n) ?? 0) + 1);
  const duplicates = [...seen.entries()].filter(([, c]) => c > 1).map(([n]) => n);

  const fc = { type: "FeatureCollection" as const, features };

  return (
    <div className="flex flex-col gap-6 sm:flex-row">
      {/* Left: map preview */}
      <div className="flex-1">
        {features.length === 0 ? (
          <div
            className={cn(
              "flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground",
            )}
            style={{ height: 300 }}
          >
            Load features first
          </div>
        ) : (
          <RecordContextProvider value={{ _preview_fc: fc }}>
            <FeatureCollectionField source="_preview_fc" testId="import-map-preview" />
          </RecordContextProvider>
        )}
      </div>

      {/* Right: naming controls */}
      <div className="flex w-full flex-col gap-4 sm:w-72">
        <div className="flex flex-col gap-1.5">
          <label htmlFor="name-prop-select" className="text-sm font-medium">
            Name property
          </label>
          <select
            id="name-prop-select"
            value={nameProp}
            onChange={(e) => setNameProp(e.target.value)}
            className="rounded-md border bg-background px-3 py-1.5 text-sm"
            aria-label="Name property"
          >
            <option value="">— none —</option>
            {propKeys.map((k) => (
              <option key={k} value={k}>
                {k}
              </option>
            ))}
          </select>
        </div>

        <div className="flex flex-col gap-1.5">
          <label htmlFor="name-template-input" className="text-sm font-medium">
            Name template
          </label>
          <input
            id="name-template-input"
            type="text"
            value={template}
            onChange={(e) => setTemplate(e.target.value)}
            className="rounded-md border bg-background px-3 py-1.5 text-sm"
            placeholder="{name}"
          />
          <p className="text-xs text-muted-foreground">
            Use <code>{"{name}"}</code> and <code>{"{index}"}</code> as tokens.
          </p>
        </div>

        {/* Warnings */}
        {(emptyCount > 0 || duplicates.length > 0) && (
          <div role="alert" className="flex flex-col gap-1 text-sm text-destructive">
            {emptyCount > 0 && (
              <span>{emptyCount} feature(s) have an empty name</span>
            )}
            {duplicates.length > 0 && (
              <span>
                {duplicates.length} duplicate name(s): {duplicates.join(", ")}
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export { MapNameStep };
