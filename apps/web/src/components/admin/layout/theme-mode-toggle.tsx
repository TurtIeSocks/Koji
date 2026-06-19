import type { Ref } from "react";
import { Check, Moon, Sun } from "lucide-react";
import { useTranslate } from "shadmin-core";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useTheme } from "@/hooks/use-theme";

/**
 * Toggle button that lets users switch between light, dark, and system UI themes.
 *
 * User's selection is persisted using the store.
 * Automatically included in the default Layout component header.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/theme-mode-toggle ThemeModeToggle documentation}
 */
interface ThemeModeToggleProps {
  ref?: Ref<HTMLButtonElement>;
}

function ThemeModeToggle({ ref }: ThemeModeToggleProps = {}) {
  const [theme, setTheme] = useTheme();
  const translate = useTranslate();

  return (
    <DropdownMenu data-theme-mode-toggle modal={false}>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="hidden sm:inline-flex relative"
          ref={ref}
        >
          <Sun className="h-[1.2rem] w-[1.2rem] rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
          <Moon className="absolute h-[1.2rem] w-[1.2rem] rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
          <span className="sr-only">
            {translate("ra.action.toggle_theme", { _: "Toggle theme" })}
          </span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onClick={() => setTheme("light")}>
          {translate("ra.theme.light", { _: "Light" })}
          <Check className={cn("ml-auto", theme !== "light" && "hidden")} />
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => setTheme("dark")}>
          {translate("ra.theme.dark", { _: "Dark" })}
          <Check className={cn("ml-auto", theme !== "dark" && "hidden")} />
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => setTheme("system")}>
          {translate("ra.theme.system", { _: "System" })}
          <Check className={cn("ml-auto", theme !== "system" && "hidden")} />
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export { ThemeModeToggle, type ThemeModeToggleProps };
