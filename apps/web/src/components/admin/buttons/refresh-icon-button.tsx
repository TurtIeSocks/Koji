import * as React from "react";
import { useCallback } from "react";
import { RefreshCw } from "lucide-react";
import { useRefresh, useTranslate } from "shadmin-core";

import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Button } from "@/components/ui/button";
import { resolveLabel } from "@/lib/resolve-label";

type RefreshIconButtonProps = {
  className?: string;
  icon?: React.ReactNode;
  label?: string;
} & Omit<React.ComponentProps<typeof Button>, "onClick"> & {
    onClick?: React.MouseEventHandler<HTMLButtonElement>;
  };

/**
 * An icon button that refreshes the current view's data.
 *
 * Shows a tooltip with the translated label. Use it in toolbars where space is limited.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/refresh-icon-button RefreshIconButton documentation}
 *
 * @example
 * import { RefreshIconButton } from '@/components/admin';
 *
 * const MyAppBar = () => (
 *   <header>
 *     <RefreshIconButton />
 *   </header>
 * );
 */
function RefreshIconButton(props: RefreshIconButtonProps) {
  const {
    label = "ra.action.refresh",
    icon = defaultIcon,
    onClick,
    className,
    ref,
    ...rest
  } = props;
  const refresh = useRefresh();
  const translate = useTranslate();
  const translatedLabel = resolveLabel(label, translate, "Refresh");

  const handleClick = useCallback(
    (event: React.MouseEvent<HTMLButtonElement>) => {
      event.preventDefault();
      refresh();
      if (typeof onClick === "function") {
        onClick(event);
      }
    },
    [refresh, onClick],
  );

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            ref={ref}
            variant="ghost"
            size="icon"
            type="button"
            aria-label={translatedLabel}
            className={className}
            onClick={handleClick}
            {...rest}
          >
            {icon}
          </Button>
        </TooltipTrigger>
        <TooltipContent>{translatedLabel}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

const defaultIcon = <RefreshCw />;

export { RefreshIconButton, type RefreshIconButtonProps };
