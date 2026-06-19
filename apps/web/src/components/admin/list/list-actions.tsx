import type { HTMLAttributes, ReactElement, ReactNode } from "react";
import {
  type Exporter,
  useFilterContext,
  useListContext,
  useResourceContext,
  useResourceDefinition,
} from "shadmin-core";
import { cn } from "@/lib/utils";
import { CreateButton } from "@/components/admin/buttons/create-button";
import { ExportButton } from "@/components/admin/buttons/export-button";
import { FilterButton } from "@/components/admin/buttons/filter-button";

interface ListActionsProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "children"> {
  className?: string;
  resource?: string;
  filters?: ReactElement | ReactNode[];
  exporter?: Exporter | boolean;
  hasCreate?: boolean;
  children?: ReactNode;
  showFilter?: (filterName: string) => void;
  displayedFilters?: Record<string, boolean>;
  filterValues?: Record<string, unknown>;
  permanentFilter?: Record<string, unknown>;
  total?: number;
}

/**
 * Default action toolbar rendered on top of `<List>`.
 *
 * If `children` are passed they replace the entire toolbar content; otherwise
 * the toolbar renders the conventional combo: `<FilterButton>` (when
 * filters are configured), `<CreateButton>` (when the resource exposes
 * a create route) and `<ExportButton>` (unless `exporter` is `false`).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/list-actions ListActions documentation}
 *
 * @example
 * import { CreateButton, ExportButton, List, ListActions } from "@/components/admin";
 *
 * const PostListActions = () => (
 *   <ListActions>
 *     <CreateButton />
 *     <ExportButton />
 *   </ListActions>
 * );
 *
 * export const PostList = () => (
 *   <List actions={<PostListActions />}>
 *     ...
 *   </List>
 * );
 */
function ListActions(props: ListActionsProps) {
  const {
    className,
    children,
    filters: filtersProp,
    exporter: exporterProp,
    hasCreate: hasCreateProp,
    resource: _resourceProp,
    showFilter: _showFilter,
    displayedFilters: _displayedFilters,
    filterValues: _filterValues,
    permanentFilter: _permanentFilter,
    total: _total,
    ...rest
  } = props;
  const resource = useResourceContext(props);
  const { hasCreate: hasCreateFromDefinition } = useResourceDefinition(props);
  const hasCreate = hasCreateProp ?? hasCreateFromDefinition;
  const filtersFromContext = useFilterContext();
  const filters = filtersProp ?? filtersFromContext;
  const { exporter: exporterFromContext } = useListContext();
  const exporter = exporterProp ?? exporterFromContext;

  return (
    <div
      className={cn("flex items-center gap-2 justify-end", className)}
      {...rest}
    >
      {children ?? (
        <>
          {filters && (Array.isArray(filters) ? filters.length > 0 : true) ? (
            <FilterButton />
          ) : null}
          {hasCreate ? <CreateButton resource={resource} /> : null}
          {exporter !== false ? <ExportButton resource={resource} /> : null}
        </>
      )}
    </div>
  );
}

export { ListActions, type ListActionsProps };
