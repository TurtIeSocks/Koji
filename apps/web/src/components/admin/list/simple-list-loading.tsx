import { useTimeout } from "shadmin-core";
import { cn } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";

interface SimpleListLoadingProps {
  /**
   * Whether to reserve a slot for a left avatar / icon in each skeleton row.
   */
  hasLeftAvatarOrIcon?: boolean;
  /**
   * Whether to reserve a slot for a right avatar / icon in each skeleton row.
   */
  hasRightAvatarOrIcon?: boolean;
  /**
   * Whether to render a secondary text placeholder under the primary one.
   */
  hasSecondaryText?: boolean;
  /**
   * Whether to render a tertiary text placeholder next to the primary one.
   */
  hasTertiaryText?: boolean;
  /**
   * Number of fake rows to render. Defaults to 5.
   */
  nbFakeLines?: number;
  className?: string;
}

/**
 * Skeleton placeholder rendered by `<SimpleList>` while data is loading.
 *
 * Mirrors the layout of `<SimpleList>` so the page does not jump when the data arrives.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/simple-list-loading SimpleListLoading documentation}
 *
 * @example
 * import { SimpleListLoading } from "@/components/admin";
 *
 * const Loading = () => (
 *   <SimpleListLoading hasLeftAvatarOrIcon hasSecondaryText nbFakeLines={8} />
 * );
 */
function SimpleListLoading({
  hasLeftAvatarOrIcon,
  hasRightAvatarOrIcon,
  hasSecondaryText,
  hasTertiaryText,
  nbFakeLines = 5,
  className,
}: SimpleListLoadingProps) {
  const oneSecondHasPassed = useTimeout(1000);
  if (!oneSecondHasPassed) return null;
  return (
    <ul className={cn("flex flex-col", className)}>
      {Array.from({ length: nbFakeLines }).map((_, key) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: fixed-count loading skeleton placeholders with no data identity and stable order
        <li key={key} className="flex items-center gap-3 px-3 py-2">
          {hasLeftAvatarOrIcon && (
            <Skeleton className="size-10 shrink-0 rounded-full" />
          )}
          <div className="flex flex-col flex-1 gap-2 min-w-0">
            <div className="flex items-center justify-between gap-2">
              <Skeleton className="h-4 w-2/3" />
              {hasTertiaryText && <Skeleton className="h-3 w-12 shrink-0" />}
            </div>
            {hasSecondaryText && <Skeleton className="h-3 w-1/3" />}
          </div>
          {hasRightAvatarOrIcon && (
            <Skeleton className="size-10 shrink-0 rounded-full" />
          )}
        </li>
      ))}
    </ul>
  );
}

export { SimpleListLoading, type SimpleListLoadingProps };
