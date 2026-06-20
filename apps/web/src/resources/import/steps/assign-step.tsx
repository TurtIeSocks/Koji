import { useEffect, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useWrappedSource } from "shadmin-core";
import {
  ArrayInput,
  AutocompleteArrayInput,
  AutocompleteInput,
  ReferenceArrayInput,
  ReferenceInput,
  SelectInput,
  SimpleFormIterator,
  TextInput,
} from "@/components/admin";
import { Button } from "@/components/ui/button";
import { GEOFENCE_MODES } from "@/lib/constants";

// ── Constants ────────────────────────────────────────────────────────────────

const KIND_CHOICES = [
  { id: "geofence", name: "Geofence" },
  { id: "route", name: "Route" },
] as const;

const COLLISION_CHOICES = [
  { id: "skip", name: "Skip" },
  { id: "overwrite", name: "Overwrite" },
] as const;

// ── Helpers ──────────────────────────────────────────────────────────────────

type GeoJsonGeometry = { type?: string } | undefined;

function isRouteGeometry(geom: GeoJsonGeometry): boolean {
  return (
    geom?.type === "MultiPoint" ||
    geom?.type === "LineString" ||
    geom?.type === "MultiLineString"
  );
}

// ── FeatureRow ────────────────────────────────────────────────────────────────
// One row inside the features iterator. Reads its own geometry via scoped
// useWrappedSource + useWatch to derive default kind, then reads the user's
// current kind choice to pick the parent field variant.

function FeatureRow() {
  const geomSource = useWrappedSource("geometry");
  const geom = useWatch({ name: geomSource }) as GeoJsonGeometry;

  const kindSource = useWrappedSource("kind");
  const kind = useWatch({ name: kindSource }) as string | undefined;

  // Derive effective kind: user choice wins, else geometry-derived default.
  const derivedKind = isRouteGeometry(geom) ? "route" : "geofence";
  const effectiveKind = kind ?? derivedKind;
  const isRoute = effectiveKind === "route";

  return (
    <div className="flex w-full flex-col gap-2">
      <div className="flex flex-wrap gap-2">
        <TextInput source="name" label="Name" />
        <SelectInput
          source="kind"
          label="Kind"
          choices={[...KIND_CHOICES]}
          defaultValue={derivedKind}
        />
        <SelectInput
          source="mode"
          label="Mode"
          choices={[...GEOFENCE_MODES]}
          defaultValue="unset"
        />
        <SelectInput
          source="on_collision"
          label="On collision"
          choices={[...COLLISION_CHOICES]}
          defaultValue="skip"
        />
      </div>
      <div className="flex flex-wrap gap-2">
        {!isRoute && (
          <ReferenceInput source="parent" reference="geofence">
            <AutocompleteInput optionText="name" label="Parent" />
          </ReferenceInput>
        )}
        {isRoute && (
          <ReferenceInput source="route_parent" reference="geofence">
            <AutocompleteInput optionText="name" label="Route parent" />
          </ReferenceInput>
        )}
        <ReferenceArrayInput source="projects" reference="project">
          <AutocompleteArrayInput label="Projects" />
        </ReferenceArrayInput>
        {/* ponytail: bulk parent / route_parent / projects apply deferred — less repetitive than mode/collision */}
      </div>
    </div>
  );
}

// ── BulkBar ───────────────────────────────────────────────────────────────────
// Small control row above the iterator for bulk-applying mode / on_collision
// to every row at once.

function BulkBar() {
  const { getValues, setValue } = useFormContext();
  const [bulkMode, setBulkMode] = useState("unset");
  const [bulkCollision, setBulkCollision] = useState("skip");

  const applyModeToAll = () => {
    const features = (getValues("features") as Record<string, unknown>[]) ?? [];
    setValue(
      "features",
      features.map((f) => ({ ...f, mode: bulkMode })),
      { shouldDirty: true },
    );
  };

  const applyCollisionToAll = () => {
    const features = (getValues("features") as Record<string, unknown>[]) ?? [];
    setValue(
      "features",
      features.map((f) => ({ ...f, on_collision: bulkCollision })),
      { shouldDirty: true },
    );
  };

  return (
    <div className="flex flex-wrap items-center gap-3 rounded-md border bg-muted/40 p-3">
      <span className="text-sm font-medium text-muted-foreground">
        Bulk apply:
      </span>

      <label className="flex items-center gap-1 text-sm">
        Mode
        <select
          aria-label="Bulk mode"
          value={bulkMode}
          onChange={(e) => setBulkMode(e.target.value)}
          className="rounded border bg-background px-2 py-1 text-sm"
        >
          {GEOFENCE_MODES.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
            </option>
          ))}
        </select>
      </label>
      <Button type="button" size="sm" variant="secondary" onClick={applyModeToAll}>
        Apply mode to all
      </Button>

      <label className="flex items-center gap-1 text-sm">
        Collision
        <select
          aria-label="Bulk collision"
          value={bulkCollision}
          onChange={(e) => setBulkCollision(e.target.value)}
          className="rounded border bg-background px-2 py-1 text-sm"
        >
          {COLLISION_CHOICES.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      </label>
      <Button
        type="button"
        size="sm"
        variant="secondary"
        onClick={applyCollisionToAll}
      >
        Apply collision to all
      </Button>
    </div>
  );
}

// ── AssignStep ────────────────────────────────────────────────────────────────

/** Step 2 — Assign mode/parent/projects/collision to each imported feature.
 *  Seeds each row's `name` from the B5 name-prop/template fields on mount,
 *  only for rows whose `name` is still empty. */
function AssignStep() {
  const { getValues, setValue } = useFormContext();

  // Name seeding — runs ONCE on mount, only fills empty names.
  useEffect(() => {
    const features = (getValues("features") as Record<string, unknown>[]) ?? [];
    const prop = (getValues("_name_prop") as string) ?? "";
    const tpl = (getValues("_name_template") as string) || "{name}";
    let changed = false;
    const seeded = features.map((f, i) => {
      if (f?.name) return f; // keep existing / user-edited name
      changed = true;
      const props = f?.properties as Record<string, unknown> | undefined;
      const nm = tpl
        .replaceAll("{name}", String(prop ? (props?.[prop] ?? "") : ""))
        .replaceAll("{index}", String(i));
      return { ...f, name: nm };
    });
    if (changed) setValue("features", seeded, { shouldDirty: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // mount-only; revisiting the step re-seeds only still-empty rows

  return (
    <div className="flex flex-col gap-4">
      <BulkBar />
      <ArrayInput source="features" label="Features">
        <SimpleFormIterator inline>
          <FeatureRow />
        </SimpleFormIterator>
      </ArrayInput>
    </div>
  );
}

export { AssignStep };
export default AssignStep;
