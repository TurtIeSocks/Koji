import { useEffect, useRef } from "react";
import {
  useEvent,
  useInfinitePaginationContext,
  useListContext,
  useTranslate,
} from "shadmin-core";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { cn } from "@/lib/utils";

interface InfinitePaginationProps {
  /**
   * Options forwarded to the underlying `IntersectionObserver` watching the
   * sentinel element. Defaults to `{ threshold: 0 }`.
   */
  options?: IntersectionObserverInit;
  className?: string;
}

const defaultOptions: IntersectionObserverInit = { threshold: 0 };

/**
 * A pagination component for `<InfiniteList>`.
 *
 * Combines an `IntersectionObserver` (which fetches the next page when the
 * sentinel scrolls into view) with a "Load more" button that lets keyboard
 * users trigger the same action explicitly.
 *
 * Must be used inside an `<InfinitePaginationContext>` — typically as a child
 * of `<InfiniteList>`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/infinite-pagination InfinitePagination documentation}
 *
 * @example
 * import { InfiniteList, InfinitePagination } from "@/components/admin";
 *
 * const PostList = () => (
 *   <InfiniteList pagination={<InfinitePagination className="py-8" />}>
 *     ...
 *   </InfiniteList>
 * );
 */
function InfinitePagination({
  options = defaultOptions,
  className,
}: InfinitePaginationProps = {}) {
  const { isPending } = useListContext();
  const { fetchNextPage, hasNextPage, isFetchingNextPage } =
    useInfinitePaginationContext();
  const translate = useTranslate();
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  const handleObserver = useEvent<[IntersectionObserverEntry[]], void>(
    (entries) => {
      const [target] = entries;
      if (target?.isIntersecting && hasNextPage && !isFetchingNextPage) {
        fetchNextPage();
      }
    },
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: isPending must stay so the effect re-runs once the sentinel mounts (the component renders null while pending, so on the first pass sentinelRef is empty and the observer cannot attach). fetchNextPage/hasNextPage/isFetchingNextPage are read through the stable useEvent handler and listed to re-attach the observer when paging state changes.
  useEffect(() => {
    const element = sentinelRef.current;
    if (!element) return;
    const observer = new IntersectionObserver(handleObserver, options);
    observer.observe(element);
    return () => observer.unobserve(element);
  }, [
    fetchNextPage,
    handleObserver,
    hasNextPage,
    isFetchingNextPage,
    isPending,
    options,
  ]);

  if (isPending || !hasNextPage) return null;

  return (
    <div
      ref={sentinelRef}
      className={cn("flex justify-center py-4", className)}
    >
      <Button
        type="button"
        variant="outline"
        onClick={() => fetchNextPage()}
        disabled={isFetchingNextPage}
      >
        {isFetchingNextPage ? (
          <>
            <Spinner aria-hidden="true" />
            {translate("ra.action.loading", { _: "Loading..." })}
          </>
        ) : (
          translate("ra.navigation.next", { _: "Load more" })
        )}
      </Button>
    </div>
  );
}

export { InfinitePagination, type InfinitePaginationProps };
