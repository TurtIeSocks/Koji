import type { ReactNode, Ref } from "react";
import { useRefresh, useLoading, useTranslate } from "shadmin-core";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { RotateCw } from "lucide-react";

const defaultIcon = <RotateCw />;

interface RefreshButtonProps {
  ref?: Ref<HTMLButtonElement>;
  label?: ReactNode;
  icon?: ReactNode;
  onClick?: () => void;
}

/**
 * A button that refreshes the current view's data.
 *
 * When clicked, reloads data from the server. Shows a spinner animation during loading.
 * Included in the default top app bar. Hidden on small screens.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/refresh-button RefreshButton documentation}
 */
function RefreshButton({
  ref,
  label,
  icon = defaultIcon,
  onClick,
}: RefreshButtonProps = {}) {
  const refresh = useRefresh();
  const loading = useLoading();
  const translate = useTranslate();

  const handleRefresh = () => {
    refresh();
    if (onClick) {
      onClick();
    }
  };

  const defaultLabel = translate("ra.action.refresh", { _: "Refresh" });
  const ariaLabel = label != null ? String(label) : defaultLabel;

  return (
    <Button
      ref={ref}
      onClick={handleRefresh}
      variant="ghost"
      size="icon"
      className="hidden sm:inline-flex"
      aria-label={ariaLabel}
      disabled={loading}
    >
      {loading ? <Spinner /> : icon}
    </Button>
  );
}

export { RefreshButton, type RefreshButtonProps };
