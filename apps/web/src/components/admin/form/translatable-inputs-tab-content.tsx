import { useMemo, type ReactNode } from "react";
import {
  FormGroupContextProvider,
  RecordContextProvider,
  SourceContextProvider,
  useRecordContext,
  useSourceContext,
  useTranslatableContext,
  type RaRecord,
} from "shadmin-core";

import { TabsContent } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

/**
 * Default container for a group of translatable inputs inside a
 * `<TranslatableInputs>` component.
 *
 * Wraps its children in:
 * - a `<FormGroupContextProvider>` named `${groupKey}${locale}` so
 *   `<TranslatableInputsTab>` can read the group's validation state,
 * - a shadcn `<TabsContent>` with `forceMount` so form state for all
 *   locales stays mounted while only the selected locale is visible,
 * - a `<SourceContextProvider>` that rewrites every child's `source`
 *   from `path` to `path.${locale}`,
 * - a `<RecordContextProvider>` with the record filtered to the current
 *   locale (so initial values resolve correctly).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/translatable-inputs TranslatableInputs documentation}
 */
function TranslatableInputsTabContent(
  props: TranslatableInputsTabContentProps,
) {
  const { children, groupKey = "", locale, className } = props;
  const { getRecordForLocale } = useTranslatableContext();
  const parentSourceContext = useSourceContext();
  const record = useRecordContext(props);

  // The SourceContext will be read by children of TranslatableInputs to
  // compute their composed source and label:
  //   <TranslatableInputs locales={['en', 'fr']}> // SourceContext is "fr"
  //     <TextInput source="description" />        // final source: "description.fr"
  //   </TranslatableInputs>
  const sourceContext = useMemo(
    () => ({
      getSource: (source: string) => {
        if (!source) {
          throw new Error("Children of TranslatableInputs must have a source");
        }
        return parentSourceContext.getSource(`${source}.${locale}`);
      },
      getLabel: (source: string) => parentSourceContext.getLabel(source),
    }),
    [locale, parentSourceContext],
  );

  // Build a record where translatable fields have their value set to the
  // value of the current locale. Necessary because the inputs use a
  // RecordContext to compute initial values and have no knowledge of the
  // locale.
  const recordForLocale = useMemo(
    () => getRecordForLocale(record, locale),
    [getRecordForLocale, record, locale],
  );

  return (
    <FormGroupContextProvider name={`${groupKey}${locale}`}>
      <TabsContent
        value={locale}
        forceMount
        id={`translatable-content-${groupKey}${locale}`}
        aria-labelledby={`translatable-header-${groupKey}${locale}`}
        className={cn(
          "flex flex-col gap-3 rounded-b-md border border-t-0 p-4 data-[state=inactive]:hidden",
          className,
        )}
      >
        <SourceContextProvider value={sourceContext}>
          <RecordContextProvider value={recordForLocale}>
            {children}
          </RecordContextProvider>
        </SourceContextProvider>
      </TabsContent>
    </FormGroupContextProvider>
  );
}

interface TranslatableInputsTabContentProps {
  children: ReactNode;
  className?: string;
  groupKey?: string;
  locale: string;
  record?: RaRecord;
  resource?: string;
}

export { TranslatableInputsTabContent, type TranslatableInputsTabContentProps };
