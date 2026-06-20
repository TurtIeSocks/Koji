import { useWatch } from "react-hook-form";
import {
  TextInput,
  NumberInput,
  BooleanInput,
  ColorInput,
} from "@/components/admin";
import { MonacoJsonInput } from "@/components/monaco";

interface PropertyValueInputProps {
  source: string;
  /**
   * The RHF field name to watch for the category value.
   * Defaults to "category".
   */
  categorySource?: string;
  /** Explicit category — overrides the watched sibling when provided. */
  category?: string;
  /** Field label. Defaults to "Default Value". */
  label?: string;
}

const JSON_CATEGORIES = new Set(["object", "array"]);

/**
 * A dynamic input that renders the appropriate input type for a property's
 * `default_value` field, switching on the sibling `category` field via
 * `useWatch`. Non-destructive on category change — the current value is kept
 * (RHF retains `default_value`; `useWatch` only triggers a re-render).
 *
 * Used by property edit/create forms and geofence properties arrays.
 *
 * @example
 * <PropertyValueInput source="default_value" />
 * <PropertyValueInput source="value" categorySource="prop_category" />
 * <PropertyValueInput source="value" category="boolean" label="Value" />
 */
function PropertyValueInput({
  source,
  categorySource = "category",
  category: categoryProp,
  label = "Default Value",
}: PropertyValueInputProps) {
  // Hooks rule: always call useWatch; ignore its result when an explicit category is given.
  const watched = useWatch({ name: categorySource }) as string | undefined;
  const category = categoryProp ?? watched;

  if (category === "boolean") {
    return <BooleanInput source={source} label={label} />;
  }

  if (category === "number") {
    return <NumberInput source={source} label={label} />;
  }

  if (category === "color") {
    return <ColorInput source={source} label={label} />;
  }

  if (category != null && JSON_CATEGORIES.has(category)) {
    return (
      <MonacoJsonInput
        source={source}
        label={label}
        height={200}
        helperText={`Must be a JSON ${category}.`}
      />
    );
  }

  if (category === "database") {
    return (
      <TextInput
        source={source}
        label={label}
        disabled
        helperText="Resolved from the database at runtime — cannot be set here."
      />
    );
  }

  // Fallback: string / unknown / undefined → plain TextInput
  return <TextInput source={source} label={label} />;
}

export { PropertyValueInput, type PropertyValueInputProps };
