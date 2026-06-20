import type { RaRecord } from "shadmin-core";
import { sanitizeFieldRestProps, useFieldValue } from "shadmin-core";
import type { FieldProps } from "@/lib/field-types";

/**
 * Displays a color value as a swatch + hex text.
 *
 * Renders a small rounded bordered circle filled with the hex color alongside
 * the hex string. Use for properties with category "color".
 *
 * @example
 * import { ColorField } from '@/components/admin';
 *
 * <ColorField source="default_value" />
 */
interface ColorFieldProps<RecordType extends RaRecord = RaRecord>
  extends FieldProps<RecordType> {}

function ColorField<RecordType extends RaRecord = RaRecord>({
  source,
  record,
  empty,
  ...rest
}: ColorFieldProps<RecordType>) {
  const value = useFieldValue({ source, record });

  if (value == null || value === "") {
    return empty ?? null;
  }

  const hex = typeof value === "string" ? value : String(value);

  return (
    <span
      className="inline-flex items-center gap-2"
      {...sanitizeFieldRestProps(rest)}
    >
      <span
        aria-hidden="true"
        className="inline-block h-4 w-4 shrink-0 rounded-full border border-border"
        style={{ backgroundColor: hex }}
      />
      <span className="font-mono text-sm">{hex}</span>
    </span>
  );
}

export { ColorField, type ColorFieldProps };
