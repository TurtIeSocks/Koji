"use client";

import type { ReactElement, ReactNode } from "react";
import { Children, Fragment, isValidElement } from "react";
import type { RaRecord } from "shadmin-core";
import { OptionalRecordContextProvider } from "shadmin-core";
import { cn } from "@/lib/utils";
import { Labeled } from "@/components/admin/views/labeled";

/**
 * A simple vertical layout for Show views that auto-labels every field child.
 *
 * Each direct child is wrapped in a `<Labeled>` component so that field labels
 * are rendered automatically from the field's `source` prop — matching
 * react-admin's `SimpleShowLayout` behaviour.
 *
 * Suppress a label by passing `label={false}` on the child field, or skip
 * auto-labeling for a specific child by wrapping it in `<Labeled label={false}>`.
 *
 * @see {@link https://marmelab.com/react-admin/SimpleShowLayout.html}
 *
 * @example
 * import { Show, SimpleShowLayout, TextField, NumberField } from '@/components/admin';
 *
 * const PostShow = () => (
 *   <Show>
 *     <SimpleShowLayout>
 *       <TextField source="title" />
 *       <TextField source="author" />
 *       <NumberField source="views" />
 *     </SimpleShowLayout>
 *   </Show>
 * );
 */
function SimpleShowLayout({
  children,
  className,
  divider,
  record,
  spacing = 1,
}: SimpleShowLayoutProps) {
  const childArray = Children.toArray(children);
  const content = (
    <div className={cn(`flex flex-col gap-${spacing}`, className)}>
      {childArray.map((child, index) => {
        const labeled = isValidElement(child) ? (
          <Labeled>{child as ReactElement}</Labeled>
        ) : (
          child
        );
        return (
          // biome-ignore lint/suspicious/noArrayIndexKey: keys arbitrary React children by position (React.Children); no stable per-child identity available
          <Fragment key={index}>
            {labeled}
            {divider != null && index < childArray.length - 1 && divider}
          </Fragment>
        );
      })}
    </div>
  );
  return record != null ? (
    <OptionalRecordContextProvider value={record}>
      {content}
    </OptionalRecordContextProvider>
  ) : (
    content
  );
}

interface SimpleShowLayoutProps {
  children?: ReactNode;
  className?: string;
  divider?: ReactNode;
  record?: RaRecord;
  spacing?: number;
}

export { SimpleShowLayout, type SimpleShowLayoutProps };
