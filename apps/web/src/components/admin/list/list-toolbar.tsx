import {
  Children,
  cloneElement,
  isValidElement,
  type ReactElement,
  type ReactNode,
} from "react";
import { type Exporter, FilterContext } from "shadmin-core";
import { cn } from "@/lib/utils";
import { FilterForm } from "@/components/admin/list/filter-form";
import { ListActions } from "@/components/admin/list/list-actions";

interface ListToolbarProps {
  filters?: ReactElement | ReactNode[];
  actions?: ReactNode | false;
  className?: string;
  exporter?: Exporter | false;
  hasCreate?: boolean;
}

/**
 * Toolbar rendered on top of `<List>` views.
 *
 * Combines an inline `<FilterForm>` on the left with a `<ListActions>` slot
 * on the right (typically holding `<FilterButton>`, `<CreateButton>` and
 * `<ExportButton>`). Custom filters can be passed as an array of inputs or
 * as a single React element — they are exposed through `<FilterContext>` so
 * the children of `<ListActions>` automatically pick them up.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/list-toolbar ListToolbar documentation}
 */
function ListToolbar(props: ListToolbarProps) {
  const { filters, actions, className, exporter, hasCreate } = props;
  let filtersArray: ReactNode[] | undefined;
  if (Array.isArray(filters)) {
    filtersArray = filters;
  } else if (isValidElement(filters)) {
    const childProps = filters.props as { children?: ReactNode };
    filtersArray = Children.toArray(childProps.children);
  }

  let actionsElement: ReactNode;
  if (actions === false) {
    actionsElement = null;
  } else if (actions == null) {
    actionsElement = <ListActions exporter={exporter} hasCreate={hasCreate} />;
  } else if (isValidElement(actions)) {
    actionsElement = cloneElement(
      actions as ReactElement<Record<string, unknown>>,
      {
        exporter,
        hasCreate,
        filters: filtersArray,
      },
    );
  } else {
    actionsElement = actions;
  }

  const content = (
    <div
      className={cn(
        "flex justify-between items-start flex-wrap gap-2 my-2",
        className,
      )}
    >
      <div className="flex-1">
        <FilterForm filters={filtersArray} />
      </div>
      {actionsElement != null ? <div>{actionsElement}</div> : null}
    </div>
  );

  if (filtersArray) {
    return (
      <FilterContext.Provider value={filtersArray}>
        {content}
      </FilterContext.Provider>
    );
  }
  return content;
}

export { ListToolbar, type ListToolbarProps };
