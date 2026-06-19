import { Children, isValidElement, useMemo, type ReactNode } from "react";
import {
  getResourceFieldLabelKey,
  RecordContextProvider,
  SourceContextProvider,
  useOptionalSourceContext,
  useResourceContext,
  useTranslatableContext,
  type RaRecord,
} from "shadmin-core";

import { Label } from "@/components/ui/label";
import { TabsContent } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

/**
 * Default container for a group of translatable fields inside a
 * `<TranslatableFields>` component.
 *
 * Provides a fresh `RecordContext` (with the record filtered to a single
 * locale) and a `SourceContext` that rewrites every child's `source` from
 * `path` to `path.${locale}`. Wrapped in a shadcn `<TabsContent>` with
 * `forceMount` so the panels stay mounted across locale switches.
 *
 * When more than one child is passed, each child is wrapped in a small
 * inline `<Label>` derived from its `source` (deviation from upstream which
 * uses MUI's `<Labeled>` — no equivalent component exists in this kit).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/translatable-fields TranslatableFields documentation}
 */
function TranslatableFieldsTabContent(
  props: TranslatableFieldsTabContentProps,
) {
  const { children, groupKey = "", locale, record, className } = props;
  const { getRecordForLocale } = useTranslatableContext();
  const addLabel = Children.count(children) > 1;

  const parentSourceContext = useOptionalSourceContext();
  const resource = useResourceContext(props);
  if (!resource) {
    throw new Error(
      `<TranslatableFieldsTabContent> was called outside of a ResourceContext and without a resource prop. You must set the resource prop.`,
    );
  }

  const sourceContext = useMemo(
    () => ({
      getSource: (source: string) =>
        parentSourceContext
          ? parentSourceContext.getSource(`${source}.${locale}`)
          : `${source}.${locale}`,
      getLabel: (source: string) =>
        parentSourceContext
          ? parentSourceContext.getLabel(source)
          : getResourceFieldLabelKey(resource, source),
    }),
    [locale, parentSourceContext, resource],
  );

  // As fields rely on the RecordContext to get their values and have no
  // knowledge of the locale, we create a new record with only the current
  // locale's values: { title: { en, fr } } => { title: <value for locale> }
  const recordForLocale = useMemo(
    () => getRecordForLocale(record, locale),
    [getRecordForLocale, record, locale],
  );

  return (
    <TabsContent
      value={locale}
      forceMount
      id={`translatable-content-${groupKey}${locale}`}
      aria-labelledby={`translatable-header-${groupKey}${locale}`}
      className={cn(
        "rounded-b-md border border-t-0 p-4 data-[state=inactive]:hidden",
        className,
      )}
    >
      <RecordContextProvider value={recordForLocale}>
        <SourceContextProvider value={sourceContext}>
          {Children.map(children, (field) =>
            field &&
            isValidElement<{ source?: string; label?: string }>(field) ? (
              <div>
                {addLabel ? (
                  <Label className="mb-1 block text-xs text-muted-foreground">
                    {field.props.label ?? field.props.source}
                  </Label>
                ) : null}
                {field}
              </div>
            ) : null,
          )}
        </SourceContextProvider>
      </RecordContextProvider>
    </TabsContent>
  );
}

interface TranslatableFieldsTabContentProps {
  children: ReactNode;
  className?: string;
  groupKey?: string;
  locale: string;
  record: RaRecord;
  resource?: string;
}

export { TranslatableFieldsTabContent, type TranslatableFieldsTabContentProps };
