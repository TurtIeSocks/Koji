import { memo } from "react";
import { FilterLiveForm, useTranslate } from "shadmin-core";
import {
  SearchInput,
  type SearchInputProps,
} from "@/components/admin/inputs/search-input";

/**
 * Form and search input for doing a full-text search filter from a sidebar.
 *
 * Triggers a search on change (with debounce). Designed to be used in the
 * aside of a `<List>` view, alongside `<FilterList>` and `<FilterListItem>`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/filter-live-search FilterLiveSearch documentation}
 *
 * @example
 * import { Card } from "@/components/ui/card";
 * import { FilterLiveSearch } from "@/components/admin";
 *
 * const FilterSidebar = () => (
 *   <Card className="p-4">
 *     <FilterLiveSearch />
 *   </Card>
 * );
 */
interface FilterLiveSearchProps extends Omit<SearchInputProps, "source"> {
  source?: string;
}

function FilterLiveSearchComponent(props: FilterLiveSearchProps) {
  const translate = useTranslate();
  const {
    source = "q",
    placeholder = translate("ra.action.search", { _: "Search" }),
    ...rest
  } = props;

  return (
    <FilterLiveForm>
      <SearchInput source={source} placeholder={placeholder} {...rest} />
    </FilterLiveForm>
  );
}

const FilterLiveSearch = memo(FilterLiveSearchComponent);

export { FilterLiveSearch, type FilterLiveSearchProps };
