import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";
import { FilterListSection } from "@/components/admin/list/filter-list-section";

/**
 * Header and container for a list of filter list items, typically rendered as
 * the aside of a `<List>` view.
 *
 * Expects 2 props, and a list of `<FilterListItem>` as children:
 *
 * - `label`: The label for this filter section. Will be translated.
 * - `icon`: An icon React element.
 *
 * @see FilterListItem
 * @see {@link https://shadmin.turtlesocks.dev/docs/filter-list FilterList documentation}
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
interface FilterListProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "children"> {
  label: string;
  icon?: ReactNode;
  children: ReactNode;
}

function FilterList(props: FilterListProps) {
  const { children, ...rest } = props;
  return (
    <FilterListSection {...rest}>
      <ul className={cn("flex flex-col gap-0.5")}>{children}</ul>
    </FilterListSection>
  );
}

export { FilterList, type FilterListProps };
