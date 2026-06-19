import type { MouseEvent, Ref } from "react";
import type { RaRecord, UseGetListOptions } from "shadmin-core";
import { Translate, useListContext } from "shadmin-core";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * A button that selects all records (up to a limit) for the current list.
 *
 * Uses ListContext.getData when available to select across all pages,
 * falling back to onSelectAll otherwise.
 *
 * To be used inside the <DataTable bulkActionsButtons> prop or in BulkActionsToolbar.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/select-all-button SelectAllButton documentation}
 *
 * @example
 * import { BulkActionsToolbar, BulkDeleteButton, SelectAllButton } from '@/components/admin';
 *
 * const PostBulkActionsToolbar = () => (
 *   <BulkActionsToolbar>
 *     <SelectAllButton />
 *     <BulkDeleteButton />
 *   </BulkActionsToolbar>
 * );
 */
function SelectAllButton<RecordType extends RaRecord = RaRecord>({
  label = "ra.action.select_all",
  limit = 250,
  queryOptions,
  className,
  onClick,
  ref,
  ...props
}: SelectAllButtonProps<RecordType>) {
  const listContext = useListContext<RecordType>();
  const { getData, onSelect, onSelectAll, selectedIds, total } = listContext;
  const data: RecordType[] | undefined = listContext.data;

  const handleClick = async (event: MouseEvent<HTMLButtonElement>) => {
    if (getData) {
      const records = await getData({
        maxResults: limit,
        meta: queryOptions?.meta,
      });
      onSelect(records?.map((record) => record.id) ?? []);
    } else {
      onSelectAll({ limit, queryOptions });
    }
    onClick?.(event);
  };

  // Hide the button when:
  // - everything is already selected (total === selectedIds.length)
  // - we've hit the selection cap
  // - not all currently loaded data is selected (button only offered as an
  //   escalation from "all on this page" to "all across pages")
  const areAllDataSelected = data?.every((item) =>
    selectedIds.includes(item.id),
  );

  if (
    total === selectedIds.length ||
    selectedIds.length >= limit ||
    !areAllDataSelected
  ) {
    return null;
  }

  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      className={cn("h-9", className)}
      onClick={handleClick}
      ref={ref}
      {...props}
    >
      <Translate i18nKey={label}>{label}</Translate>
    </Button>
  );
}

type SelectAllButtonProps<RecordType extends RaRecord = RaRecord> = {
  label?: string;
  limit?: number;
  queryOptions?: UseGetListOptions<RecordType>;
  ref?: Ref<HTMLButtonElement>;
} & React.ComponentPropsWithoutRef<"button">;

export { SelectAllButton, type SelectAllButtonProps };
