import * as React from "react";
import { useTranslate } from "shadmin-core";
import { capitalize } from "inflection";

import { TabsTrigger } from "@/components/ui/tabs";

/**
 * Single tab that selects a locale in a `<TranslatableFields>` component.
 *
 * Renders as a shadcn `<TabsTrigger>` whose label is the translated locale
 * name (e.g. `ra.locales.en`), falling back to the capitalized locale code
 * when no translation key is found.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/translatable-fields TranslatableFields documentation}
 *
 * @example
 * <TranslatableFieldsTab locale="fr" />
 */
function TranslatableFieldsTab(props: TranslatableFieldsTabProps) {
  const { locale, groupKey = "", ...rest } = props;
  const translate = useTranslate();

  return (
    <TabsTrigger
      id={`translatable-header-${groupKey}${locale}`}
      value={locale}
      {...rest}
    >
      {translate(`ra.locales.${groupKey}${locale}`, {
        _: capitalize(locale),
      })}
    </TabsTrigger>
  );
}

interface TranslatableFieldsTabProps
  extends Omit<React.ComponentProps<typeof TabsTrigger>, "value"> {
  locale: string;
  groupKey?: string;
}

export { TranslatableFieldsTab, type TranslatableFieldsTabProps };
