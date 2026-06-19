import type { Ref } from "react";
import { useTranslate } from "shadmin-core";
import { SidebarTrigger, useSidebar } from "@/components/ui/sidebar";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface SidebarToggleButtonProps {
  className?: string;
  ref?: Ref<HTMLButtonElement>;
}

/**
 * A standalone button that opens or closes the main sidebar.
 *
 * Convenience wrapper around the shadcn/ui `<SidebarTrigger>` matching the
 * sizing applied by the default `<Layout>` header. Use it when composing
 * a custom layout that needs to expose the sidebar toggle outside the
 * default header slot.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/sidebar-toggle-button SidebarToggleButton documentation}
 *
 * @example
 * import { SidebarToggleButton } from "@/components/admin";
 *
 * const CustomHeader = () => (
 *   <header>
 *     <SidebarToggleButton />
 *   </header>
 * );
 */
function SidebarToggleButton(props: SidebarToggleButtonProps) {
  const { className, ref } = props;
  const { open } = useSidebar();
  const translate = useTranslate();
  const tooltipText = open
    ? translate("ra.action.close_menu", { _: "Close menu" })
    : translate("ra.action.open_menu", { _: "Open menu" });

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <SidebarTrigger
          className={cn("scale-125 sm:scale-100", className)}
          ref={ref}
        />
      </TooltipTrigger>
      <TooltipContent>{tooltipText}</TooltipContent>
    </Tooltip>
  );
}

export { SidebarToggleButton, type SidebarToggleButtonProps };
