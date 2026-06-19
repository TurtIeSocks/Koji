import type { ReactNode, Ref } from "react";
import { Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useLocales, useLocaleState, useTranslate } from "shadmin-core";

/**
 * Language switcher button that displays a menu allowing users to select the interface language.
 *
 * Automatically renders in the header when multiple locales are configured in the i18nProvider.
 * User's language selection is persisted using the store.
 * Returns null if only one language is available.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/locales-menu-button LocalesMenuButton documentation}
 * @see {@link https://marmelab.com/ra-core/translationsetup/ i18nProvider setup}
 */
interface LocalesMenuButtonProps {
  ref?: Ref<HTMLButtonElement>;
  /** Replaces the locale-code text in the trigger button when provided. */
  icon?: ReactNode;
  /** Override the list of available languages instead of calling `useLocales()`. */
  languages?: Array<{ locale: string; name: string }>;
}

function LocalesMenuButton({
  ref,
  icon,
  languages: languagesProp,
}: LocalesMenuButtonProps = {}) {
  const localesFromHook = useLocales();
  const languages = languagesProp ?? localesFromHook;
  const [locale, setLocale] = useLocaleState();
  const translate = useTranslate();

  const getNameForLocale = (locale: string): string => {
    const language = languages.find((language) => language.locale === locale);
    return language ? language.name : "";
  };

  const changeLocale = (locale: string) => (): void => {
    setLocale(locale);
  };

  if (languages.length <= 1) {
    return null; // No need to render the dropdown if there's only one language
  }
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="hidden sm:inline-flex"
          aria-label={translate("ra.action.change_locale", {
            _: "Change locale",
          })}
          ref={ref}
        >
          {icon ?? locale.toUpperCase()}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {languages.map((language) => (
          <DropdownMenuItem
            key={language.locale}
            onClick={changeLocale(language.locale)}
          >
            {getNameForLocale(language.locale)}
            <Check
              className={cn("ml-auto", locale !== language.locale && "hidden")}
            />
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export { LocalesMenuButton, type LocalesMenuButtonProps };
