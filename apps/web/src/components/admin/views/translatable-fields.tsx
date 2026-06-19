import type { ReactElement, ReactNode } from "react";
import {
  TranslatableContextProvider,
  useRecordContext,
  useResourceContext,
  useTranslatable,
  type RaRecord,
  type UseTranslatableOptions,
} from "shadmin-core";

import { Tabs } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { TranslatableFieldsTabContent } from "@/components/admin/views/translatable-fields-tab-content";
import { TranslatableFieldsTabs } from "@/components/admin/views/translatable-fields-tabs";

/**
 * Show multiple language values for any field passed as children. Children
 * automatically get their `source` rewritten to `source.${locale}` via a
 * `SourceContext`, so plain `<TextField source="name" />` works.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/translatable-fields TranslatableFields documentation}
 */
function TranslatableFields(props: TranslatableFieldsProps) {
  const {
    defaultLocale,
    locales,
    groupKey = "",
    selector = <TranslatableFieldsTabs groupKey={groupKey} />,
    children,
    className,
    resource: resourceProp,
  } = props;

  const record = useRecordContext(props);
  if (!record) {
    throw new Error(
      `<TranslatableFields> was called outside of a RecordContext and without a record prop. You must set the record prop.`,
    );
  }

  const resource = useResourceContext(props);
  if (!resource) {
    throw new Error(
      `<TranslatableFields> was called outside of a ResourceContext and without a resource prop. You must set the resource prop.`,
    );
  }

  const context = useTranslatable({ defaultLocale, locales });

  return (
    <TranslatableContextProvider value={context}>
      <Tabs
        value={context.selectedLocale}
        onValueChange={context.selectLocale}
        className={cn("w-full", className)}
      >
        {selector}
        {locales.map((locale) => (
          <TranslatableFieldsTabContent
            key={locale}
            locale={locale}
            record={record}
            resource={resourceProp}
            groupKey={groupKey}
          >
            {children}
          </TranslatableFieldsTabContent>
        ))}
      </Tabs>
    </TranslatableContextProvider>
  );
}

interface TranslatableFieldsProps extends UseTranslatableOptions {
  children: ReactNode;
  className?: string;
  record?: RaRecord;
  resource?: string;
  selector?: ReactElement;
  groupKey?: string;
}

export { TranslatableFields, type TranslatableFieldsProps };
