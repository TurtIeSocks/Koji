import { useTranslatableContext } from "shadmin-core";

import { TabsList } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { TranslatableFieldsTab } from "@/components/admin/views/translatable-fields-tab";

/**
 * Default locale selector for the `<TranslatableFields>` component.
 *
 * Reads the list of locales from the `TranslatableContext` and renders one
 * `<TranslatableFieldsTab>` per locale, wrapped in a shadcn `<TabsList>`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/translatable-fields TranslatableFields documentation}
 *
 * @example
 * <TranslatableFields
 *     locales={['en', 'fr']}
 *     selector={<TranslatableFieldsTabs />}
 * >
 *     <TextField source="name" />
 * </TranslatableFields>
 */
function TranslatableFieldsTabs(props: TranslatableFieldsTabsProps) {
  const { groupKey, className } = props;
  const { locales } = useTranslatableContext();

  return (
    <TabsList
      className={cn("w-full justify-start rounded-b-none h-auto", className)}
    >
      {locales.map((locale) => (
        <TranslatableFieldsTab
          key={locale}
          locale={locale}
          groupKey={groupKey}
        />
      ))}
    </TabsList>
  );
}

interface TranslatableFieldsTabsProps {
  groupKey?: string;
  className?: string;
}

export { TranslatableFieldsTabs, type TranslatableFieldsTabsProps };
