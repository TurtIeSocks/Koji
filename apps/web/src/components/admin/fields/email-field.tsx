import {
  genericMemo,
  sanitizeFieldRestProps,
  useFieldValue,
  useTranslate,
} from "shadmin-core";
import type { AnchorHTMLAttributes } from "react";
import React from "react";

import { cn } from "@/lib/utils";
import type { FieldProps } from "@/lib/field-types";
import type { UnknownRecord } from "@/lib/unknown-types";

function EmailFieldImpl<RecordType extends UnknownRecord = UnknownRecord>(
  inProps: EmailFieldProps<RecordType>,
) {
  const { className, empty, defaultValue, source, record, ...rest } = inProps;
  const value = useFieldValue({ defaultValue, source, record });
  const translate = useTranslate();

  if (value == null) {
    if (!empty) {
      return null;
    }

    return (
      <span className={className} {...sanitizeFieldRestProps(rest)}>
        {typeof empty === "string" ? translate(empty, { _: empty }) : empty}
      </span>
    );
  }

  return (
    <a
      className={cn("underline hover:no-underline", className)}
      href={`mailto:${value}`}
      onClick={stopPropagation}
      {...sanitizeFieldRestProps(rest)}
    >
      {value}
    </a>
  );
}
EmailFieldImpl.displayName = "EmailFieldImpl";

/**
 * Displays an email address as a clickable mailto link.
 *
 * Click events are prevented from bubbling up, making it safe to use in DataTable rows with rowClick.
 * To be used with RecordField or DataTable.Col components, or anywhere a RecordContext is available.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/email-field EmailField documentation}
 *
 * @example
 * import { List, DataTable, EmailField } from '@/components/admin';
 *
 * const UserList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col source="name" />
 *       <DataTable.Col source="email" field={EmailField} />
 *     </DataTable>
 *   </List>
 * );
 */
const EmailField = genericMemo(EmailFieldImpl);

interface EmailFieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends FieldProps<RecordType>,
    AnchorHTMLAttributes<HTMLAnchorElement> {}

// useful to prevent click bubbling in a DataTable with rowClick
const stopPropagation = (e: React.MouseEvent<HTMLAnchorElement>) =>
  e.stopPropagation();

export { EmailField, type EmailFieldProps };
