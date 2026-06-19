import { useLoading, useTranslate } from "shadmin-core";
import { cn } from "@/lib/utils";
import { Spinner } from "@/components/ui/spinner";
import { RefreshIconButton } from "@/components/admin/buttons/refresh-icon-button";

interface LoadingIndicatorProps {
  className?: string;
  onClick?: () => void;
}

/**
 * Small inline spinner that appears whenever the data provider has a
 * query or a mutation in flight.
 *
 * Backed by ra-core's `useLoading` hook. When idle, renders a
 * `<RefreshIconButton>` so the user can manually trigger a refresh.
 * When loading, shows a spinner with the refresh button overlaid at
 * opacity-0 so the layout does not shift.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/loading-indicator LoadingIndicator documentation}
 */
function LoadingIndicator(props: LoadingIndicatorProps) {
  const { className, onClick } = props;
  const loading = useLoading();
  const translate = useTranslate();

  return (
    <div className={cn("relative size-4", className)}>
      <RefreshIconButton
        onClick={onClick}
        className={cn(
          "size-4 p-0 transition-opacity",
          loading ? "opacity-0" : "opacity-100",
        )}
      />
      {loading && (
        <Spinner
          aria-label={translate("ra.page.loading", { _: "Loading" })}
          className="absolute inset-0 text-muted-foreground"
        />
      )}
    </div>
  );
}

export { LoadingIndicator, type LoadingIndicatorProps };
