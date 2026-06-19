import {
  genericMemo,
  sanitizeFieldRestProps,
  useFieldValue,
  useTranslate,
} from "shadmin-core";
import type { HTMLAttributes } from "react";
import DOMPurify from "dompurify";
import type { Config as DOMPurifyConfig } from "dompurify";
import parse from "html-react-parser";

import { cn } from "@/lib/utils";
import type { FieldProps } from "@/lib/field-types";
import type { UnknownRecord } from "@/lib/unknown-types";

// We only support the case where sanitize() returns a string,
// hence we force the RETURN_DOM_FRAGMENT and RETURN_DOM options to false.
type PurifyOptions = DOMPurifyConfig & {
  RETURN_DOM_FRAGMENT?: false | undefined;
  RETURN_DOM?: false | undefined;
};

interface RichTextFieldProps<RecordType extends UnknownRecord = UnknownRecord>
  extends FieldProps<RecordType>,
    Omit<HTMLAttributes<HTMLSpanElement>, "children"> {
  stripTags?: boolean;
  purifyOptions?: PurifyOptions;
  /**
   * @deprecated Use the `empty` prop instead.
   */
  emptyText?: string;
}

/**
 * Strips all HTML tags by sanitizing with an empty allow-list.
 * The output is plain text safe to render as a React child.
 */
const stripHtmlTags = (input: string): string => {
  if (!input) return "";
  return DOMPurify.sanitize(input, { ALLOWED_TAGS: [], ALLOWED_ATTR: [] });
};

const RichTextFieldImpl = <RecordType extends UnknownRecord = UnknownRecord>(
  inProps: RichTextFieldProps<RecordType>,
) => {
  const {
    className,
    emptyText,
    empty,
    stripTags = false,
    purifyOptions,
    defaultValue,
    source,
    record,
    ...rest
  } = inProps;
  const value = useFieldValue({ defaultValue, source, record });
  const translate = useTranslate();

  if (value == null || value === "") {
    const fallback = empty ?? emptyText;
    if (!fallback) {
      return (
        <span className={cn(className)} {...sanitizeFieldRestProps(rest)} />
      );
    }
    return (
      <span className={cn(className)} {...sanitizeFieldRestProps(rest)}>
        {typeof fallback === "string"
          ? translate(fallback, { _: fallback })
          : fallback}
      </span>
    );
  }

  const stringValue = String(value);

  if (stripTags) {
    return (
      <span className={cn(className)} {...sanitizeFieldRestProps(rest)}>
        {stripHtmlTags(stringValue)}
      </span>
    );
  }

  // Sanitize the HTML, then parse the sanitized string into a React element tree
  // via html-react-parser. No React unsafe HTML escape hatch is used.
  const sanitized = DOMPurify.sanitize(stringValue, purifyOptions ?? {});

  return (
    <span className={cn(className)} {...sanitizeFieldRestProps(rest)}>
      {parse(sanitized as string)}
    </span>
  );
};
RichTextFieldImpl.displayName = "RichTextFieldImpl";

/**
 * Renders an HTML string as rich text after sanitizing it.
 *
 * The HTML is sanitized with DOMPurify and parsed into a React element tree with
 * `html-react-parser`, so no React HTML-injection escape hatch is used. Even so,
 * it is still good practice to sanitize the content server-side as a
 * defense-in-depth measure.
 *
 * Pass `stripTags` to render the value as plain text with all markup removed.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/rich-text-field RichTextField documentation}
 *
 * @example
 * import { Show, RecordField, RichTextField } from '@/components/admin';
 *
 * const PostShow = () => (
 *   <Show>
 *     <RecordField source="body">
 *       <RichTextField source="body" />
 *     </RecordField>
 *   </Show>
 * );
 *
 * @example // strip all HTML tags
 * <RichTextField source="body" stripTags />
 */
const RichTextField = genericMemo(RichTextFieldImpl);

export { RichTextField, type RichTextFieldProps, type PurifyOptions };
