import {
  type ComponentProps,
  type MouseEventHandler,
  type ReactNode,
  type Ref,
} from "react";
import { useTranslate, usePreferencesEditor } from "shadmin-core";
import { Settings } from "lucide-react";

const defaultIcon = <Settings className="size-4" />;

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

/**
 * Icon button that toggles the preferences edit mode on or off.
 *
 * When clicked while edit mode is disabled, calls `enable()` from
 * {@link usePreferencesEditor} which makes every {@link Configurable} in the
 * page show its hover affordance and outline. When clicked while edit mode is
 * enabled, calls `disable()` and clears the currently selected preference key.
 *
 * Renders a ghost icon `<Button>` with a {@link Settings} icon and a tooltip
 * showing the translated label.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/inspector Inspector documentation}
 *
 * @example
 * import { InspectorButton } from "@/components/admin";
 *
 * const UserMenu = () => (
 *   <header>
 *     <InspectorButton />
 *   </header>
 * );
 */
function InspectorButton({
  icon = defaultIcon,
  label = "ra.configurable.configureMode",
  ref,
  ...props
}: InspectorButtonProps) {
  const { enable, disable, setPreferenceKey, isEnabled } =
    usePreferencesEditor();
  const translate = useTranslate();

  const handleClick: MouseEventHandler<HTMLButtonElement> = (event) => {
    props.onClick?.(event);
    if (event.defaultPrevented) return;
    if (isEnabled) {
      disable();
      setPreferenceKey(null);
    } else {
      enable();
    }
  };

  const translatedLabel = translate(label, { _: "Configure mode" });

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            ref={ref}
            variant="ghost"
            size="icon"
            aria-label={translatedLabel}
            aria-pressed={isEnabled}
            {...props}
            onClick={handleClick}
          >
            {icon}
          </Button>
        </TooltipTrigger>
        <TooltipContent>{translatedLabel}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

interface InspectorButtonProps
  extends Omit<ComponentProps<typeof Button>, "children"> {
  /** Custom icon node. Defaults to the {@link Settings} icon. */
  icon?: ReactNode;
  /** Translation key for the tooltip and aria-label. */
  label?: string;
  ref?: Ref<HTMLButtonElement>;
}

export { InspectorButton, type InspectorButtonProps };
