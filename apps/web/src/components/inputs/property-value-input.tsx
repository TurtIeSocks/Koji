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
 */
function PropertyValueInput({
  source,
  categorySource = "category",
}: PropertyValueInputProps) {
  const category = useWatch({ name: categorySource }) as string | undefined;

  if (category === "boolean") {
    return <BooleanInput source={source} label="Default Value" />;
  }

  if (category === "number") {
    return <NumberInput source={source} label="Default Value" />;
  }

  if (category === "color") {
    return <ColorInput source={source} label="Default Value" />;
  }

  if (category != null && JSON_CATEGORIES.has(category)) {
    return (
      <MonacoJsonInput
        source={source}
        label="Default Value"
        height={200}
        helperText={`Must be a JSON ${category}.`}
      />
    );
  }

  if (category === "database") {
    return (
      <TextInput
        source={source}
        label="Default Value"
        disabled
        helperText="Resolved from the database at runtime — cannot be set here."
      />
    );
  }

  // Fallback: string / unknown / undefined → plain TextInput
  return <TextInput source={source} label="Default Value" />;
}

export { PropertyValueInput, type PropertyValueInputProps };
