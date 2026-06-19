import * as React from "react";
import type { HtmlHTMLAttributes, ReactNode } from "react";
import { useCallback, useEffect, isValidElement } from "react";
import get from "lodash/get";
import {
  FilterLiveForm,
  useFilterContext,
  useListContext,
  useResourceContext,
  useTranslate,
} from "shadmin-core";
import { MinusCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import type { UnknownValue } from "@/lib/unknown-types";

interface FilterElementProps {
  source: string;
  alwaysOn?: boolean;
  defaultValue?: UnknownValue;
  size?: string;
  label?: React.ReactNode;
  disabled?: boolean;
  resource?: string;
  record?: Record<string, UnknownValue>;
  helperText?: React.ReactNode | false;
}

/**
 * A form for filter inputs with live updates. Included by default in List.
 *
 * To be used in conjunction with FilterButton.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/list#filter-button--form-combo FilterForm documentation}
 */
function FilterForm(inProps: FilterFormProps) {
  const { filters: filtersProps, ...rest } = inProps;
  const filters = useFilterContext() || filtersProps;

  return (
    <FilterLiveForm formComponent={StyledForm} {...sanitizeRestProps(rest)}>
      <FilterFormBase filters={filters} />
    </FilterLiveForm>
  );
}

interface FilterFormProps extends FilterFormBaseProps {}

/**
 * @deprecated Use FilterFormBase from `ra-core` once available.
 */
function FilterFormBase(props: FilterFormBaseProps) {
  const { filters } = props;
  const resource = useResourceContext(props);
  const { displayedFilters = {}, filterValues, hideFilter } = useListContext();

  useEffect(() => {
    if (!filters) return;
    filters
      .filter(
        (
          filterElement,
        ): filterElement is React.ReactElement<FilterElementProps> =>
          isValidElement(filterElement),
      )
      .forEach((filter) => {
        if (filter.props.alwaysOn && filter.props.defaultValue) {
          throw new Error(
            "Cannot use alwaysOn and defaultValue on a filter input. Please set the filterDefaultValues props on the <List> element instead.",
          );
        }
      });
  }, [filters]);

  const getShownFilters = () => {
    if (!filters) return [];
    const values = filterValues;
    return filters
      .filter(
        (
          filterElement,
        ): filterElement is React.ReactElement<FilterElementProps> =>
          isValidElement(filterElement),
      )
      .filter((filterElement) => {
        const filterValue = get(values, filterElement.props.source);
        return (
          filterElement.props.alwaysOn ||
          displayedFilters[filterElement.props.source] ||
          !isEmptyValue(filterValue)
        );
      });
  };

  const handleHide = useCallback(
    (event: React.MouseEvent<HTMLElement>) =>
      hideFilter(event.currentTarget.dataset.key!),
    [hideFilter],
  );

  return (
    <>
      {getShownFilters().map((filterElement) => (
        <FilterFormInput
          key={filterElement.key || filterElement.props.source}
          filterElement={filterElement}
          handleHide={handleHide}
          resource={resource}
        />
      ))}
    </>
  );
}

const sanitizeRestProps = ({
  hasCreate: _hasCreate,
  resource: _resource,
  ...props
}: Partial<FilterFormBaseProps> & { hasCreate?: boolean }) => props;

type FilterFormBaseProps = Omit<
  HtmlHTMLAttributes<HTMLFormElement>,
  "children"
> & {
  className?: string;
  resource?: string;
  filters?: ReactNode[];
};

function StyledForm(props: React.FormHTMLAttributes<HTMLFormElement>) {
  return (
    <form
      {...props}
      className={cn(
        "flex flex-col sm:flex-row justify-start items-end gap-x-2 gap-y-3 pointer-events-none flex-wrap",
        "[&_.form-helper-text]:hidden",
        props.className,
      )}
    />
  );
}

const isEmptyValue = (filterValue: UnknownValue): boolean => {
  if (filterValue === "" || filterValue == null) return true;

  // If one of the value leaf is not empty
  // the value is considered not empty
  if (typeof filterValue === "object") {
    return Object.keys(filterValue).every((key) =>
      isEmptyValue(filterValue[key as keyof typeof filterValue]),
    );
  }

  return false;
};

function FilterFormInput(inProps: FilterFormInputProps) {
  const { filterElement, handleHide, className } = inProps;
  const resource = useResourceContext(inProps);
  const translate = useTranslate();

  return (
    <div
      data-source={filterElement.props.source}
      className={cn(
        "filter-field flex flex-row pointer-events-auto gap-2 relative w-full sm:w-auto",
        className,
      )}
    >
      {React.cloneElement(filterElement, {
        resource,
        record: emptyRecord,
        size: filterElement.props.size ?? "small",
        helperText: false,
        // ignore defaultValue in Field because it was already set in Form (via mergedInitialValuesWithDefaultValues)
        defaultValue: undefined,
      })}
      {!filterElement.props.alwaysOn && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="hide-filter size-9 cursor-pointer mt-auto"
          onClick={handleHide}
          data-key={filterElement.props.source}
          title={translate("ra.action.remove_filter")}
        >
          <MinusCircle className="size-4" />
        </Button>
      )}
    </div>
  );
}

interface FilterFormInputProps {
  filterElement: React.ReactElement<FilterElementProps>;
  handleHide: (event: React.MouseEvent<HTMLElement>) => void;
  className?: string;
  resource?: string;
}

const emptyRecord = {};

export {
  FilterForm,
  type FilterFormProps,
  FilterFormBase,
  type FilterFormBaseProps,
  FilterFormInput,
  type FilterFormInputProps,
};
