import { useMemo, type HTMLAttributes, type ReactNode } from "react";
import { sanitizeFieldRestProps, useRecordContext } from "shadmin-core";

import { cn } from "@/lib/utils";
import type { FieldProps } from "@/lib/field-types";
import type { UnknownRecord } from "@/lib/unknown-types";

/**
 * Field rendering its value with a custom render function.
 *
 * Useful for composing multiple record fields, or for displaying transformed data.
 * To be used with RecordField or DataTable.Col components, or anywhere a RecordContext is available.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/function-field FunctionField documentation}
 *
 * @example
 * import { List, DataTable, FunctionField } from '@/components/admin';
 *
 * const UserList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col label="Name">
 *         <FunctionField
 *           source="last_name"
 *           render={(record) => `${record.first_name} ${record.last_name}`}
 *         />
 *       </DataTable.Col>
 *     </DataTable>
 *   </List>
 * );
 */
interface FunctionFieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends Omit<FieldProps<RecordType>, "source">,
    Omit<HTMLAttributes<HTMLSpanElement>, "children"> {
  source?: string;
  render: (record: RecordType, source?: string) => ReactNode;
}

function FunctionField<RecordType extends UnknownRecord = UnknownRecord>(
  props: FunctionFieldProps<RecordType>,
) {
  const { className, source = "", render, record: recordProp, ...rest } = props;
  const record = useRecordContext<RecordType>({ record: recordProp });
  return useMemo(
    () =>
      record ? (
        <span className={cn(className)} {...sanitizeFieldRestProps(rest)}>
          {render(record, source)}
        </span>
      ) : null,
    // biome-ignore lint/correctness/useExhaustiveDependencies: rest is a rest-spread object (fresh identity each render) but is genuinely read inside the memo via sanitizeFieldRestProps; it must stay so the rendered span reflects the latest pass-through props.
    [className, record, source, render, rest],
  );
}

export { FunctionField, type FunctionFieldProps };
