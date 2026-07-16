import { useState, useEffect } from "react";
import { useWatch, useFormContext } from "react-hook-form";
import { DeckGeoJsonInput } from "@/components/deck";
import { importToFeatures, importFromFeatures } from "../import-features";

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
  // Set by the Source step on a successful load. Distinguishes "never loaded
  // anything" (show the empty-state box) from "loaded, then deleted every
  // shape on the map" (keep the editor mounted so a replacement can be drawn
  // without navigating back to Source).
  const sourceLoaded = (useWatch({ name: "_source_loaded" }) as boolean) ?? false;
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

  return (
    <div className="flex flex-col gap-6">
      {/* Naming controls — a row above the full-width map. */}
      <div className="flex flex-wrap items-end gap-4">
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

        {(emptyCount > 0 || duplicates.length > 0) && (
          <div role="alert" className="flex flex-col gap-1 text-sm text-destructive">
            {emptyCount > 0 && <span>{emptyCount} feature(s) have an empty name</span>}
            {duplicates.length > 0 && (
              <span>
                {duplicates.length} duplicate name(s): {duplicates.join(", ")}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Full-width editable map: adjust the imported shapes before saving —
          this replaces v1's "send to map for further editing" (spec D1).
          transform/split/cutHole are excluded: they'd change feature count
          or identity out from under the per-row assignments. */}
      {features.length === 0 && !sourceLoaded ? (
        <div
          className="flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground"
          style={{ height: 640 }}
        >
          Load features first
        </div>
      ) : (
        <DeckGeoJsonInput
          source="features"
          label="Preview & adjust"
          height={640}
          toFeatures={importToFeatures}
          fromFeatures={importFromFeatures}
          allowedModes={["drawPolygon", "drawRectangle", "drawCircle", "modify"]}
        />
      )}
    </div>
  );
}

export { MapNameStep };
