import type { HTMLAttributes } from "react";
import {
  sanitizeFieldRestProps,
  useFieldValue,
  useTranslate,
} from "shadmin-core";
import type { FieldProps } from "@/lib/field-types";
import type { UnknownRecord } from "@/lib/unknown-types";

/**
 * Displays a text value from a record field inside a span element.
 *
 * This is the default field component used in DataTable columns and RecordField components.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/text-field TextField documentation}
 *
 * @example
 * import { List, DataTable, TextField } from '@/components/admin';
 *
 * export const UserList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col source="id" />
 *       <DataTable.Col>
 *         <TextField source="name" empty="resources.users.fields.name.empty" />
 *       </DataTable.Col>
 *     </DataTable>
 *   </List>
 * );
 */
function TextField<RecordType extends UnknownRecord = UnknownRecord>({
  defaultValue,
  source,
  record,
  empty,
  ...rest
}: TextFieldProps<RecordType>) {
  const value = useFieldValue({ defaultValue, source, record });
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

  return (
    <span {...sanitizeFieldRestProps(rest)}>
      {typeof value !== "string" ? value.toString() : value}
    </span>
  );
}

interface TextFieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends FieldProps<RecordType>,
    HTMLAttributes<HTMLSpanElement> {}

export { TextField, type TextFieldProps };
