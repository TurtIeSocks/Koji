import { useTranslatableContext } from "shadmin-core";

import { TabsList } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { TranslatableInputsTab } from "@/components/admin/form/translatable-inputs-tab";

/**
 * Default locale selector for the `<TranslatableInputs>` component.
 *
 * Reads the list of locales from the `TranslatableContext` and renders one
 * `<TranslatableInputsTab>` per locale, wrapped in a shadcn `<TabsList>`.
 * Each tab reflects the validation state of the inputs for its locale.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/translatable-inputs TranslatableInputs documentation}
 *
 * @example
 * <TranslatableInputs
 *     locales={['en', 'fr']}
 *     selector={<TranslatableInputsTabs />}
 * >
 *     <TextInput source="name" />
 * </TranslatableInputs>
 */
function TranslatableInputsTabs(props: TranslatableInputsTabsProps) {
  const { groupKey, className } = props;
  const { locales } = useTranslatableContext();

  return (
    <TabsList
      className={cn("w-full justify-start rounded-b-none h-auto", className)}
    >
      {locales.map((locale) => (
        <TranslatableInputsTab
          key={locale}
          locale={locale}
          groupKey={groupKey}
        />
      ))}
    </TabsList>
  );
}

interface TranslatableInputsTabsProps {
  groupKey?: string;
  className?: string;
}

export { TranslatableInputsTabs, type TranslatableInputsTabsProps };
