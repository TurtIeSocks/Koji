import type { HTMLAttributes, ReactNode } from "react";
import { useTranslate } from "shadmin-core";
import { cn } from "@/lib/utils";

/**
 * Wrapper to render children inside a filter list section.
 *
 * Adds a header with an icon and a (translated) label before rendering the
 * children. Used internally by `<FilterList>`, but can also be used standalone
 * to make custom widgets look nicer alongside a filter list.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/filter-list-section FilterListSection documentation}
 *
 * @example
 * import { Card } from "@/components/ui/card";
 * import { Type } from "lucide-react";
 * import {
 *   FilterListSection,
 *   FilterLiveForm,
 *   TextInput,
 * } from "@/components/admin";
 *
 * export const BookListAside = () => (
 *   <Card className="p-4">
 *     <FilterListSection label="Title" icon={<Type />}>
 *       <FilterLiveForm>
 *         <TextInput source="title" />
 *       </FilterLiveForm>
 *     </FilterListSection>
 *   </Card>
 * );
 */
interface FilterListSectionProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "children"> {
  label: string;
  icon?: ReactNode;
  children: ReactNode;
}

function FilterListSection(props: FilterListSectionProps) {
  const { label, icon, children, className, ...rest } = props;
  const translate = useTranslate();
  return (
    <div className={cn("mt-4 first:mt-0", className)} {...rest}>
      <div className="flex flex-row items-center gap-2 mb-2 text-sm font-medium">
        {icon ? <span className="flex shrink-0">{icon}</span> : null}
        <span>{translate(label, { _: label })}</span>
      </div>
      {children}
    </div>
  );
}

export { FilterListSection, type FilterListSectionProps };
