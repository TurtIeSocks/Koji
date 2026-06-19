import { cn } from "@/lib/utils";
import { buttonVariants } from "@/components/ui/button";
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
} from "@/components/ui/pagination";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ChevronLeftIcon, ChevronRightIcon } from "lucide-react";
import {
  useListPaginationContext,
  Translate,
  useTranslate,
} from "shadmin-core";
import type { ReactElement } from "react";
import { useEffect } from "react";

/**
 * A pagination component with page numbers and rows per page selector.
 *
 * Displays pagination controls with previous/next buttons, page numbers with ellipsis for long lists,
 * and a dropdown to change items per page. Works with List context.
 *
 * When the total number of records is known, displays page numbers with ellipsis
 * for long lists ("1-25 of 312"). When the total is unknown (partial pagination
 * from cursor-based / infinite providers), displays simplified prev/next buttons
 * with a range indicator only ("1-25").
 *
 * Returns `null` when the list is empty (total === 0).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/list-pagination ListPagination documentation}
 *
 * @example
 * import { List, ListPagination } from '@/components/admin';
 *
 * const PostListPagination = () => (
 *   <ListPagination rowsPerPageOptions={[5, 10, 25]} />
 * );
 *
 * export const PostList = () => (
 *   <List pagination={<PostListPagination />}>
 *     // ...
 *   </List>
 * );
 */
interface ListPaginationProps {
  actions?: ReactElement;
  rowsPerPageOptions?: number[];
  className?: string;
}

function ListPagination({
  actions,
  rowsPerPageOptions = [5, 10, 25, 50],
  className,
}: ListPaginationProps) {
  const translate = useTranslate();
  const {
    hasPreviousPage,
    hasNextPage,
    page,
    perPage,
    setPerPage,
    total,
    setPage,
  } = useListPaginationContext();

  // Partial pagination = total is unknown (cursor-based / infinite providers).
  const isPartial = total == null;

  // Out-of-bounds guard: if the current page is beyond the last page, reset to 1.
  const totalPages =
    !isPartial && total! > 0 ? Math.ceil(total! / perPage) : undefined;
  useEffect(() => {
    if (totalPages != null && page > totalPages) {
      setPage(1);
    }
  }, [page, totalPages, setPage]);
  if (totalPages != null && page > totalPages) {
    return null;
  }

  // Issue 1: when the total is known and equals 0, render nothing.
  // No pagination is needed for an empty list. The empty state is handled
  // by the component displaying the data (Datagrid, DataTable, SimpleList).
  if (!isPartial && total === 0) {
    return null;
  }

  // Custom actions override the default pagination controls.
  if (actions) {
    return (
      <div
        className={cn("flex items-center justify-end gap-x-2 gap-4", className)}
      >
        {actions}
      </div>
    );
  }

  const pageStart = (page - 1) * perPage + 1;

  // Issue 3: avoid "undefined" in the range info when total is unknown.
  // When total is known, clamp the end to `total` so the last page shows the
  // real number of items rather than `page * perPage`.
  const pageEnd =
    total == null ? page * perPage : Math.min(page * perPage, total);

  // Issue 2: page-number/ellipsis logic only makes sense when total is known.
  // We compute it lazily below and guard the JSX with `!isPartial`.
  const count = total != null && total > 0 ? Math.ceil(total / perPage) : 1;

  const boundaryCount = 1;
  const siblingCount = 1;

  const range = (start: number, end: number) => {
    const length = end - start + 1;
    return Array.from({ length }, (_, i) => start + i);
  };

  const startPages = range(1, Math.min(boundaryCount, count));
  const endPages = range(
    Math.max(count - boundaryCount + 1, boundaryCount + 1),
    count,
  );

  const siblingsStart = Math.max(
    Math.min(
      // Natural start
      page - siblingCount,
      // Lower boundary when page is high
      count - boundaryCount - siblingCount * 2 - 1,
    ),
    // Greater than startPages
    boundaryCount + 2,
  );

  const siblingsEnd = Math.min(
    Math.max(
      // Natural end
      page + siblingCount,
      // Upper boundary when page is low
      boundaryCount + siblingCount * 2 + 2,
    ),
    // Less than endPages
    count - boundaryCount - 1,
  );

  const siblingPages = range(siblingsStart, siblingsEnd);

  const goToPage = (newPage: number) => setPage(newPage);

  const previousLabel = translate("ra.navigation.previous", { _: "Previous" });
  const nextLabel = translate("ra.navigation.next", { _: "Next" });

  // Issue 6: use <button type="button"> semantics for in-page navigation
  // (no href, no fake "#" anchor). Styled with the same shadcn button variants
  // that PaginationLink uses so the visual is identical.
  // Issue 4: a single element is rendered for prev/next regardless of disabled
  // state so tab order is preserved; we use aria-disabled and pointer-events-none
  // when disabled rather than swapping the DOM element.
  const navButtonClass = (active = false, disabled = false) =>
    cn(
      buttonVariants({
        variant: active ? "outline" : "ghost",
        size: "icon",
      }),
      disabled && "pointer-events-none opacity-50 cursor-not-allowed",
    );

  return (
    <div
      className={cn("flex items-center justify-end gap-x-2 gap-4", className)}
    >
      <div className="hidden md:flex items-center gap-x-2">
        <p className="text-sm font-medium">
          <Translate i18nKey="ra.navigation.page_rows_per_page">
            Rows per page
          </Translate>
        </p>
        <Select
          value={perPage.toString()}
          onValueChange={(value) => {
            setPerPage(Number(value));
          }}
        >
          <SelectTrigger className="h-8 w-fit">
            <SelectValue placeholder={perPage} />
          </SelectTrigger>
          <SelectContent side="top">
            {rowsPerPageOptions.map((pageSize) => (
              <SelectItem key={pageSize} value={`${pageSize}`}>
                {pageSize}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="text-sm text-muted-foreground">
        {isPartial ? (
          <Translate
            i18nKey="ra.navigation.partial_page_range_info"
            options={{
              offsetBegin: pageStart,
              offsetEnd: pageEnd,
            }}
          >
            {`${pageStart}-${pageEnd}`}
          </Translate>
        ) : (
          <Translate
            i18nKey="ra.navigation.page_range_info"
            options={{
              offsetBegin: pageStart,
              offsetEnd: pageEnd,
              total: total === -1 ? pageEnd : total,
            }}
          >
            {`${pageStart}-${pageEnd} of ${total === -1 ? pageEnd : total}`}
          </Translate>
        )}
      </div>
      <Pagination className="-w-full -mx-auto">
        <PaginationContent>
          <PaginationItem>
            <button
              type="button"
              aria-label={previousLabel}
              aria-disabled={!hasPreviousPage || undefined}
              disabled={!hasPreviousPage}
              onClick={hasPreviousPage ? () => goToPage(page - 1) : undefined}
              className={navButtonClass(false, !hasPreviousPage)}
            >
              <ChevronLeftIcon />
            </button>
          </PaginationItem>
          {!isPartial && (
            <>
              {startPages.map((pageNumber) => (
                <PaginationItem key={pageNumber}>
                  <button
                    type="button"
                    onClick={() => goToPage(pageNumber)}
                    aria-current={pageNumber === page ? "page" : undefined}
                    className={navButtonClass(pageNumber === page)}
                  >
                    {pageNumber}
                  </button>
                </PaginationItem>
              ))}
              {siblingsStart > boundaryCount + 2 ? (
                <PaginationItem>
                  <PaginationEllipsis />
                </PaginationItem>
              ) : boundaryCount + 1 < count - boundaryCount ? (
                <PaginationItem>
                  <button
                    type="button"
                    onClick={() => goToPage(boundaryCount + 1)}
                    aria-current={
                      boundaryCount + 1 === page ? "page" : undefined
                    }
                    className={navButtonClass(boundaryCount + 1 === page)}
                  >
                    {boundaryCount + 1}
                  </button>
                </PaginationItem>
              ) : null}
              {siblingPages.map((pageNumber) => (
                <PaginationItem key={pageNumber}>
                  <button
                    type="button"
                    onClick={() => goToPage(pageNumber)}
                    aria-current={pageNumber === page ? "page" : undefined}
                    className={navButtonClass(pageNumber === page)}
                  >
                    {pageNumber}
                  </button>
                </PaginationItem>
              ))}
              {siblingsEnd < count - boundaryCount - 1 ? (
                <PaginationItem>
                  <PaginationEllipsis />
                </PaginationItem>
              ) : count - boundaryCount > boundaryCount ? (
                <PaginationItem>
                  <button
                    type="button"
                    onClick={() => goToPage(count - boundaryCount)}
                    aria-current={
                      count - boundaryCount === page ? "page" : undefined
                    }
                    className={navButtonClass(count - boundaryCount === page)}
                  >
                    {count - boundaryCount}
                  </button>
                </PaginationItem>
              ) : null}
              {endPages.map((pageNumber) => (
                <PaginationItem key={pageNumber}>
                  <button
                    type="button"
                    onClick={() => goToPage(pageNumber)}
                    aria-current={pageNumber === page ? "page" : undefined}
                    className={navButtonClass(pageNumber === page)}
                  >
                    {pageNumber}
                  </button>
                </PaginationItem>
              ))}
            </>
          )}
          <PaginationItem>
            <button
              type="button"
              aria-label={nextLabel}
              aria-disabled={!hasNextPage || undefined}
              disabled={!hasNextPage}
              onClick={hasNextPage ? () => goToPage(page + 1) : undefined}
              className={navButtonClass(false, !hasNextPage)}
            >
              <ChevronRightIcon />
            </button>
          </PaginationItem>
        </PaginationContent>
      </Pagination>
    </div>
  );
}

export { ListPagination, type ListPaginationProps };
