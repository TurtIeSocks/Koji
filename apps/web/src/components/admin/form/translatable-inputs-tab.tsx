import * as React from "react";
import { useFormGroup, useTranslate } from "shadmin-core";
import { capitalize } from "inflection";

import { TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

/**
 * Single tab that selects a locale in a `<TranslatableInputs>` component.
 * Highlights in `text-destructive` when its form group has invalid inputs.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/translatable-inputs TranslatableInputs documentation}
 */
function TranslatableInputsTab(props: TranslatableInputsTabProps) {
  const { groupKey = "", locale, className, ...rest } = props;
  const { isValid } = useFormGroup(`${groupKey}${locale}`);
  const translate = useTranslate();

  return (
    <TabsTrigger
      id={`translatable-header-${groupKey}${locale}`}
      value={locale}
      className={cn(!isValid && "text-destructive", className)}
      {...rest}
    >
      {translate(`ra.locales.${locale}`, {
        _: capitalize(locale),
      })}
    </TabsTrigger>
  );
}

interface TranslatableInputsTabProps
  extends Omit<React.ComponentProps<typeof TabsTrigger>, "value"> {
  groupKey?: string;
  locale: string;
}

export { TranslatableInputsTab, type TranslatableInputsTabProps };
