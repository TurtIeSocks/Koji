import {
  extractValidSavedQueries,
  useListContext,
  useSavedQueries,
  useTranslate,
} from "shadmin-core";
import type {
  ChangeEvent,
  ComponentProps,
  FormEvent,
  ReactElement,
  ReactNode,
} from "react";
import { useState } from "react";
import isEqual from "lodash/isEqual";
import { Bookmark, MinusCircle, PlusCircle, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { FilterList } from "@/components/admin/list/filter-list";

/**
 * Dialog for saving the current list view query (filters, sort, pagination) as a named saved query.
 *
 * Used by FilterButton in the List view.
 *
 * @internal
 */
function AddSavedQueryDialog({
  open,
  onClose,
}: AddSavedQueryDialogProps): ReactElement {
  const translate = useTranslate();
  const { resource, filterValues, displayedFilters, sort, perPage } =
    useListContext();

  const [savedQueries, setSavedQueries] = useSavedQueries(resource);

  // input state
  const [queryName, setQueryName] = useState("");
  const handleQueryNameChange = (
    event: ChangeEvent<HTMLInputElement>,
  ): void => {
    setQueryName(event.target.value);
  };

  const handleFormSubmit = (e: FormEvent<HTMLFormElement>): void => {
    e.preventDefault();
    addQuery();
  };

  const addQuery = (): void => {
    const newSavedQuery = {
      label: queryName,
      value: {
        filter: filterValues,
        sort,
        perPage,
        displayedFilters,
      },
    };
    const newSavedQueries = extractValidSavedQueries(savedQueries);
    setSavedQueries(newSavedQueries.concat(newSavedQuery));
    setQueryName("");
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>
            {translate("ra.saved_queries.new_dialog_title", {
              _: "Save current query as",
            })}
          </DialogTitle>
          <DialogDescription>
            {translate("ra.saved_queries.new_dialog_description", {
              _: "Save the current filters and sort under a name to recall them later.",
            })}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleFormSubmit} className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="name">
              {translate("ra.saved_queries.query_name", {
                _: "Query name",
              })}
            </Label>
            <Input
              id="name"
              value={queryName}
              onChange={handleQueryNameChange}
              autoFocus
            />
          </div>
        </form>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {translate("ra.action.cancel")}
          </Button>
          <Button onClick={addQuery}>{translate("ra.action.save")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface AddSavedQueryDialogProps {
  open: boolean;
  onClose: () => void;
}

/**
 * Dialog for removing a saved query from the user's list.
 *
 * @internal
 */
function RemoveSavedQueryDialog({
  open,
  onClose,
}: RemoveSavedQueryDialogProps): ReactElement {
  const translate = useTranslate();
  const { resource, filterValues, sort, perPage, displayedFilters } =
    useListContext();

  const [savedQueries, setSavedQueries] = useSavedQueries(resource);

  const removeQuery = (): void => {
    const savedQueryToRemove = {
      filter: filterValues,
      sort,
      perPage,
      displayedFilters,
    };

    const newSavedQueries = extractValidSavedQueries(savedQueries);
    const index = newSavedQueries.findIndex((savedFilter) =>
      isEqual(savedFilter.value, savedQueryToRemove),
    );
    if (index === -1) {
      onClose();
      return;
    }
    setSavedQueries([
      ...newSavedQueries.slice(0, index),
      ...newSavedQueries.slice(index + 1),
    ]);
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {translate("ra.saved_queries.remove_dialog_title", {
              _: "Remove saved query?",
            })}
          </DialogTitle>
          <DialogDescription>
            {translate("ra.saved_queries.remove_message", {
              _: "Are you sure you want to remove that item from your list of saved queries?",
            })}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {translate("ra.action.cancel")}
          </Button>
          <Button onClick={removeQuery} autoFocus>
            {translate("ra.action.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface RemoveSavedQueryDialogProps {
  open: boolean;
  onClose: () => void;
}

type IconButtonProps = Omit<ComponentProps<typeof Button>, "children">;

/**
 * Standalone icon button that opens the {@link AddSavedQueryDialog}.
 *
 * Drop into a list toolbar to let users save the current query (filters,
 * sort, pagination) under a name. Renders a small plus-circle icon.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/saved-queries SavedQueries documentation}
 */
function AddSavedQueryIconButton({ className, ...rest }: IconButtonProps) {
  const [open, setOpen] = useState(false);
  const translate = useTranslate();
  return (
    <>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        aria-label={translate("ra.saved_queries.new_label", {
          _: "Save current query...",
        })}
        onClick={() => setOpen(true)}
        className={cn("size-8", className)}
        {...rest}
      >
        <PlusCircle className="size-4" />
      </Button>
      <AddSavedQueryDialog open={open} onClose={() => setOpen(false)} />
    </>
  );
}

/**
 * Standalone icon button that opens the {@link RemoveSavedQueryDialog}.
 *
 * Drop into a list toolbar (typically alongside a `<SavedQueriesList>`
 * trigger) to let users delete the currently-selected saved query.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/saved-queries SavedQueries documentation}
 */
function RemoveSavedQueryIconButton({ className, ...rest }: IconButtonProps) {
  const [open, setOpen] = useState(false);
  const translate = useTranslate();
  return (
    <>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        aria-label={translate("ra.saved_queries.remove_label", {
          _: "Remove saved query",
        })}
        onClick={() => setOpen(true)}
        className={cn("size-8", className)}
        {...rest}
      >
        <MinusCircle className="size-4" />
      </Button>
      <RemoveSavedQueryDialog open={open} onClose={() => setOpen(false)} />
    </>
  );
}

/**
 * FilterList-style sidebar component listing all saved queries for the current
 * resource. Each item applies its stored filters/sort/perPage on click, and has
 * a remove button. An "Add current query" shortcut appears at the bottom when
 * the current filter state hasn't already been saved.
 *
 * Rendered inside a `<FilterList>` wrapper so it fits naturally alongside other
 * filter sidebar sections.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/saved-queries SavedQueries documentation}
 *
 * @example
 * import { Card } from "@/components/ui/card";
 * import { SavedQueriesList, FilterList, FilterListItem } from "@/components/admin";
 *
 * const Sidebar = () => (
 *   <Card className="p-4">
 *     <SavedQueriesList />
 *     <FilterList label="Category" icon={<CategoryIcon />}>
 *       ...
 *     </FilterList>
 *   </Card>
 * );
 */
function SavedQueriesList({
  icon = <Bookmark className="size-4" />,
}: SavedQueriesListProps): ReactElement {
  const translate = useTranslate();
  const {
    resource,
    filterValues,
    displayedFilters,
    sort,
    perPage,
    setFilters,
    setPage,
    setSort,
    setPerPage,
  } = useListContext();

  const [savedQueries, setSavedQueries] = useSavedQueries(resource);
  const [addOpen, setAddOpen] = useState(false);
  const validSavedQueries = extractValidSavedQueries(savedQueries);

  const hasSavedCurrentQuery = validSavedQueries.some((q) =>
    isEqual(q.value, { filter: filterValues, sort, perPage, displayedFilters }),
  );
  const hasFilterValues = !isEqual(filterValues, {});

  const handleApply = (index: number) => {
    const query = validSavedQueries[index];
    if (!query) return;
    setFilters(query.value.filter, query.value.displayedFilters ?? {});
    if (query.value.sort) setSort(query.value.sort);
    if (query.value.perPage) setPerPage(query.value.perPage);
    setPage(1);
  };

  const handleRemove = (index: number) => {
    const newQueries = [
      ...validSavedQueries.slice(0, index),
      ...validSavedQueries.slice(index + 1),
    ];
    setSavedQueries(newQueries);
  };

  return (
    <>
      <FilterList
        label={translate("ra.saved_queries.label", { _: "Saved queries" })}
        icon={icon}
      >
        {validSavedQueries.map((query, index) => {
          const isSelected = isEqual(query.value, {
            filter: filterValues,
            sort,
            perPage,
            displayedFilters,
          });
          return (
            <li
              key={JSON.stringify(query.value)}
              className={cn(
                "flex items-stretch list-none",
                isSelected && "filter-list-item-selected",
              )}
            >
              <button
                type="button"
                onClick={() => handleApply(index)}
                aria-pressed={isSelected}
                data-selected={isSelected ? "true" : "false"}
                className={cn(
                  "flex flex-row items-center justify-between gap-2 flex-1 px-2 py-1 text-sm text-left rounded cursor-pointer",
                  "hover:bg-accent",
                  isSelected && "bg-secondary font-medium",
                )}
              >
                <span className="truncate">{query.label}</span>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-5 ml-auto text-muted-foreground opacity-70 hover:opacity-100 shrink-0"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleRemove(index);
                  }}
                  aria-label={translate("ra.saved_queries.remove_label", {
                    _: "Remove saved query",
                  })}
                  tabIndex={-1}
                >
                  <Trash2 className="size-3" />
                </Button>
              </button>
            </li>
          );
        })}
        {!hasSavedCurrentQuery && hasFilterValues && (
          <li className="flex items-stretch list-none">
            <button
              type="button"
              onClick={() => setAddOpen(true)}
              className="flex flex-row items-center gap-2 flex-1 px-2 py-1 text-sm text-left rounded cursor-pointer hover:bg-accent"
            >
              <span className="flex shrink-0">
                <PlusCircle className="size-3" />
              </span>
              <span className="truncate">
                {translate("ra.saved_queries.new_label", {
                  _: "Save current query...",
                })}
              </span>
            </button>
          </li>
        )}
      </FilterList>
      <AddSavedQueryDialog open={addOpen} onClose={() => setAddOpen(false)} />
    </>
  );
}

interface SavedQueriesListProps {
  icon?: ReactNode;
}

export {
  AddSavedQueryDialog,
  type AddSavedQueryDialogProps,
  RemoveSavedQueryDialog,
  type RemoveSavedQueryDialogProps,
  AddSavedQueryIconButton,
  RemoveSavedQueryIconButton,
  SavedQueriesList,
  type SavedQueriesListProps,
};
