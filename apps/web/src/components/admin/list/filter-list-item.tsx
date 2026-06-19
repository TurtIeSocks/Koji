import * as React from "react";
import { memo, type LiHTMLAttributes, type ReactNode } from "react";
import {
  useTranslate,
  useListFilterContext,
  shallowEqual,
  useEvent,
} from "shadmin-core";
import matches from "lodash/matches";
import pickBy from "lodash/pickBy";
import { CircleX } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { UnknownRecord } from "@/lib/unknown-types";

const isElement = (value: unknown): value is React.ReactElement =>
  React.isValidElement(value);

const DefaultIsSelected = (value: UnknownRecord, filters: UnknownRecord) =>
  matches(pickBy(value, (val) => typeof val !== "undefined"))(filters);

const DefaultToggleFilter = (value: UnknownRecord, filters: UnknownRecord) => {
  const isSelected = matches(
    pickBy(value, (val) => typeof val !== "undefined"),
  )(filters);

  if (isSelected) {
    const keysToRemove = Object.keys(value);
    return Object.keys(filters).reduce((acc, key) => {
      if (!keysToRemove.includes(key)) {
        acc[key] = filters[key];
      }
      return acc;
    }, {} as UnknownRecord);
  }

  return { ...filters, ...value };
};

const arePropsEqual = (
  prevProps: FilterListItemProps,
  nextProps: FilterListItemProps,
) =>
  prevProps.label === nextProps.label &&
  shallowEqual(prevProps.value, nextProps.value);

/**
 * Button to enable/disable a list filter inside a `<FilterList>` sidebar.
 *
 * Expects 2 props:
 *
 * - `label`: The text (or React element) to be displayed for this item. If it
 *   is a string, the component will translate it.
 * - `value`: An object to be merged into the filter value when enabling the
 *   filter (e.g. `{ is_published: true, published_at_gte: "2020-07-08" }`).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/filter-list-item FilterListItem documentation}
 *
 * @example
 * import { Card } from "@/components/ui/card";
 * import { Mail } from "lucide-react";
 * import { FilterList, FilterListItem } from "@/components/admin";
 *
 * const FilterSidebar = () => (
 *   <Card className="p-4">
 *     <FilterList label="Subscribed to newsletter" icon={<Mail />}>
 *       <FilterListItem label="Yes" value={{ has_newsletter: true }} />
 *       <FilterListItem label="No" value={{ has_newsletter: false }} />
 *     </FilterList>
 *   </Card>
 * );
 */
interface FilterListItemProps
  extends Omit<LiHTMLAttributes<HTMLLIElement>, "value" | "children"> {
  label: ReactNode;
  value: UnknownRecord;
  icon?: ReactNode;
  toggleFilter?: (
    value: UnknownRecord,
    filters: UnknownRecord,
  ) => UnknownRecord;
  isSelected?: (value: UnknownRecord, filters: UnknownRecord) => boolean;
}

function FilterListItemComponent(props: FilterListItemProps) {
  const {
    label,
    value,
    icon,
    isSelected: getIsSelected = DefaultIsSelected,
    toggleFilter: userToggleFilter = DefaultToggleFilter,
    className,
    ...rest
  } = props;
  const { filterValues, setFilters } = useListFilterContext();
  const translate = useTranslate();
  const toggleFilter = useEvent(userToggleFilter);

  // We can't wrap this function with useEvent as it is called in the render phase
  const isSelected = getIsSelected(value, filterValues);

  const handleClick = () => setFilters(toggleFilter(value, filterValues));

  const renderedLabel =
    typeof label === "string" && !isElement(label)
      ? translate(label, { _: label })
      : label;

  return (
    <li
      className={cn(
        "flex items-stretch list-none",
        isSelected && "filter-list-item-selected",
        className,
      )}
      {...rest}
    >
      <button
        type="button"
        onClick={handleClick}
        aria-pressed={isSelected}
        data-selected={isSelected ? "true" : "false"}
        className={cn(
          "flex flex-row items-center justify-between gap-2 flex-1 px-2 py-1 text-sm text-left rounded cursor-pointer",
          "hover:bg-accent",
          isSelected && "bg-secondary font-medium",
        )}
      >
        <span className="flex flex-row items-center gap-2 min-w-0">
          {icon ? <span className="flex shrink-0">{icon}</span> : null}
          <span className="truncate min-w-0">{renderedLabel}</span>
        </span>
        {isSelected && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-5 ml-auto text-muted-foreground opacity-70 hover:opacity-100"
            onClick={(event) => {
              event.stopPropagation();
              handleClick();
            }}
            tabIndex={-1}
          >
            <CircleX className="size-3" />
          </Button>
        )}
      </button>
    </li>
  );
}

const FilterListItem = memo(FilterListItemComponent, arePropsEqual);

export { FilterListItem, type FilterListItemProps };
