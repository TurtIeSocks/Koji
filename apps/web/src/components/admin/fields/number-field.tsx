import type { HTMLAttributes } from "react";
import {
  sanitizeFieldRestProps,
  useFieldValue,
  useTranslate,
} from "shadmin-core";
import type { FieldProps } from "@/lib/field-types";
import type { UnknownRecord, UnknownValue } from "@/lib/unknown-types";

/**
 * Displays a numeric value with locale-specific formatting.
 *
 * This field automatically formats numbers according to the user's locale using Intl.NumberFormat.
 * It supports custom locales and formatting options for currencies, percentages, and more.
 * To be used with RecordField or DataTable.Col components, or anywhere a RecordContext is available.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/number-field NumberField documentation}
 *
 * @example
 * import { Show, RecordField, NumberField } from '@/components/admin';
 *
 * const ProductShow = () => (
 *   <Show>
 *     <div className="flex flex-col gap-4">
 *       <RecordField source="reference" />
 *       <RecordField source="price">
 *         <NumberField source="price" options={{ style: 'currency', currency: 'USD' }} />
 *       </RecordField>
 *     </div>
 *   </Show>
 * );
 */
function NumberField<RecordType extends UnknownRecord = UnknownRecord>({
  defaultValue,
  source,
  record,
  empty,
  transform = defaultTransform,
  locales,
  options,
  ...rest
}: NumberFieldProps<RecordType>) {
  let value = useFieldValue({ defaultValue, source, record });
  const translate = useTranslate();

  if (value == null) {
    if (!empty) {
      return null;
    }

    return (
      <span {...sanitizeFieldRestProps(rest)}>
        {typeof empty === "string" ? translate(empty, { _: empty }) : empty}
      </span>
    );
  }

  if (transform) {
    value = transform(value);
  }

  return (
    <span {...sanitizeFieldRestProps(rest)}>
      {hasNumberFormat && typeof value === "number"
        ? value.toLocaleString(locales, options)
        : value}
    </span>
  );
}

interface NumberFieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends FieldProps<RecordType>,
    HTMLAttributes<HTMLSpanElement> {
  locales?: string | string[];
  options?: object;
  transform?: (value: UnknownValue) => UnknownValue;
}

const defaultTransform = (value: UnknownValue): UnknownValue =>
  value && typeof value === "string" && !Number.isNaN(Number(value))
    ? +value
    : value;

const hasNumberFormat = !!(
  typeof Intl === "object" &&
  Intl &&
  typeof Intl.NumberFormat === "function"
);

export { NumberField, type NumberFieldProps };
