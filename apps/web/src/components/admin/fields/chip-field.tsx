import {
  genericMemo,
  sanitizeFieldRestProps,
  useFieldValue,
  useTranslate,
} from "shadmin-core";
import type { ComponentProps } from "react";

import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { FieldProps } from "@/lib/field-types";
import type { UnknownRecord } from "@/lib/unknown-types";

type BadgeProps = ComponentProps<typeof Badge>;

function ChipFieldImpl<RecordType extends UnknownRecord = UnknownRecord>(
  inProps: ChipFieldProps<RecordType>,
) {
  const {
    className,
    empty,
    defaultValue,
    source,
    record,
    variant = "outline",
    ...rest
  } = inProps;
  const value = useFieldValue({ defaultValue, source, record });
  const translate = useTranslate();

  if (value == null || value === "") {
    if (!empty) {
      return null;
    }

    return (
      <span className={cn("text-muted-foreground", className)}>
        {typeof empty === "string" ? translate(empty, { _: empty }) : empty}
      </span>
    );
  }

  return (
    <Badge
      variant={variant}
      className={cn(className)}
      {...sanitizeFieldRestProps(rest)}
    >
      {typeof value !== "string" ? value.toString() : value}
    </Badge>
  );
}
ChipFieldImpl.displayName = "ChipFieldImpl";

/**
 * Displays a value inside a styled badge component, similar to a Material UI Chip.
 *
 * Useful for highlighting status values, tags, or categorical information.
 * To be used with RecordField or DataTable.Col components, or anywhere a RecordContext is available.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/chip-field ChipField documentation}
 *
 * @example
 * import { List, DataTable, ChipField } from '@/components/admin';
 *
 * const PostList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col source="title" />
 *       <DataTable.Col>
 *         <ChipField source="category" />
 *       </DataTable.Col>
 *     </DataTable>
 *   </List>
 * );
 */
const ChipField = genericMemo(ChipFieldImpl);

interface ChipFieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends FieldProps<RecordType>,
    Omit<BadgeProps, "children"> {
  variant?: "default" | "outline" | "secondary" | "destructive";
}

export { ChipField, type ChipFieldProps };
