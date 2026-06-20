import { useMemo } from "react";
import { useWatch } from "react-hook-form";
import { useGetList, useWrappedSource } from "shadmin-core";
import {
  ArrayInput,
  SimpleFormIterator,
  ReferenceInput,
  AutocompleteInput,
} from "@/components/admin";
import { PropertyValueInput } from "@/components/inputs/property-value-input";

interface PropertyRecord {
  id: number;
  category: string;
}

/** One property row: a property selector + a value input typed by the
 *  selected property's category (resolved from `categoryById`). */
function PropertyRow({
  categoryById,
}: {
  categoryById: Map<number, string>;
}) {
  // Scoped path for this iterator row, e.g. "properties.0.property_id".
  const propertyIdSource = useWrappedSource("property_id");
  const propertyId = useWatch({ name: propertyIdSource }) as
    | number
    | undefined;
  const category =
    propertyId != null ? categoryById.get(Number(propertyId)) : undefined;

  return (
    <div className="flex w-full flex-col gap-2 sm:flex-row sm:items-start">
      <ReferenceInput source="property_id" reference="property">
        <AutocompleteInput optionText="name" label="Property" />
      </ReferenceInput>
      <PropertyValueInput source="value" category={category} label="Value" />
    </div>
  );
}

/** Array input for a geofence's custom properties. Each row links an existing
 *  Property and edits its value with a category-typed widget. */
function GeofencePropertiesInput() {
  const { data } = useGetList<PropertyRecord>("property", {
    pagination: { page: 1, perPage: 1000 },
    sort: { field: "name", order: "ASC" },
    filter: {},
  });

  const categoryById = useMemo(
    () => new Map((data ?? []).map((p) => [Number(p.id), p.category])),
    [data],
  );

  return (
    <ArrayInput source="properties" label="Properties">
      <SimpleFormIterator inline>
        <PropertyRow categoryById={categoryById} />
      </SimpleFormIterator>
    </ArrayInput>
  );
}

export { GeofencePropertiesInput };
export default GeofencePropertiesInput;
