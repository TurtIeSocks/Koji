import {
  genericMemo,
  sanitizeFieldRestProps,
  useFieldValue,
  useTranslate,
} from "shadmin-core";
import type { AnchorHTMLAttributes, ReactNode } from "react";
import * as React from "react";

import { cn } from "@/lib/utils";
import type { FieldProps } from "@/lib/field-types";
import type { UnknownRecord, UnknownValue } from "@/lib/unknown-types";

/**
 * Displays a URL as a clickable hyperlink.
 *
 * Click events are prevented from bubbling up, making it safe to use in DataTable rows with rowClick.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/url-field UrlField documentation}
 *
 * @example
 * import { List, DataTable, UrlField } from '@/components/admin';
 *
 * const WebsiteList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col source="id" />
 *       <DataTable.Col source="website">
 *         <UrlField source="website" empty="No website available" />
 *       </DataTable.Col>
 *     </DataTable>
 *   </List>
 * );
 */
const UrlFieldImpl = <RecordType extends UnknownRecord = UnknownRecord>(
  inProps: UrlFieldProps<RecordType>,
) => {
  const {
    empty,
    className,
    defaultValue,
    source,
    record,
    resource: _,
    target,
    rel,
    content,
    ...rest
  } = inProps;
  const value = useFieldValue({ defaultValue, source, record });
  const translate = useTranslate();

  if (value == null || value === "") {
    if (!empty) {
      return null;
    }

    return (
      <span className={className} {...sanitizeFieldRestProps(rest)}>
        {typeof empty === "string" ? translate(empty, { _: empty }) : empty}
      </span>
    );
  }

  if (!isSafeUrl(value)) {
    // Refuse to render a clickable anchor for potentially unsafe schemes
    // (e.g. `javascript:`), but still surface the value as plain text.
    return (
      <span className={className} {...sanitizeFieldRestProps(rest)}>
        {value}
      </span>
    );
  }

  // When opening in a new tab, default to a secure rel value unless the
  // caller has explicitly provided their own.
  const safeRel =
    target === "_blank" && rel == null ? "noopener noreferrer" : rel;

  return (
    <a
      className={cn("underline hover:no-underline", className)}
      onClick={stopPropagation}
      {...sanitizeFieldRestProps(rest)}
      href={value}
      target={target}
      rel={safeRel}
    >
      {content ?? value}
    </a>
  );
};
UrlFieldImpl.displayName = "UrlFieldImpl";

const UrlField = genericMemo(UrlFieldImpl);

interface UrlFieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends FieldProps<RecordType>,
    Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "content"> {
  /** Content to display as the link text. Defaults to the URL value. */
  content?: ReactNode;
}

export { UrlField, type UrlFieldProps };

// useful to prevent click bubbling in a DataTable with rowClick
const stopPropagation = (e: React.MouseEvent<HTMLAnchorElement>) =>
  e.stopPropagation();

// Allowlist of safe URL schemes / prefixes. Prevents `javascript:`,
// `data:` and other XSS-prone URLs from being rendered as live links.
const SAFE_URL_PATTERN = /^(https?:|mailto:|tel:|\/|#)/i;

const isSafeUrl = (value: UnknownValue): value is string => {
  if (typeof value !== "string") return false;
  return SAFE_URL_PATTERN.test(value.trim());
};
