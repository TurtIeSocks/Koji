import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbPage,
} from "@/components/admin/layout/breadcrumb";
import type {
  ListBaseProps,
  ListControllerResult,
  RaRecord,
} from "shadmin-core";
import {
  FilterContext,
  ListBase,
  Translate,
  useGetResourceLabel,
  useHasDashboard,
  useListContext,
  useResourceContext,
  useResourceDefinition,
  useTranslate,
} from "shadmin-core";
import type { ElementType, ReactElement, ReactNode } from "react";
import { Link } from "react-router";
import { cn } from "@/lib/utils";
import { CreateButton } from "@/components/admin/buttons/create-button";
import { Empty } from "@/components/admin/feedback/empty";
import { ExportButton } from "@/components/admin/buttons/export-button";
import { ListPagination } from "@/components/admin/list/list-pagination";
import { FilterButton } from "@/components/admin/buttons/filter-button";
import { FilterForm } from "@/components/admin/list/filter-form";

/**
 * A complete list page with breadcrumb, title, filters, and pagination.
 *
 * It fetches a list of records from the data provider (via ra-core hooks),
 * puts them in a ListContext, renders a default layout (breadcrumb, title,
 * action buttons, inline filters, pagination), then renders its children
 * (usually a <DataTable>).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/list List documentation}
 *
 * @example
 * import { DataTable, List } from "@/components/admin";
 *
 * export const UserList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col source="id" />
 *       <DataTable.Col source="name" />
 *       <DataTable.Col source="username" />
 *       <DataTable.Col source="email" />
 *       <DataTable.Col source="address.street" />
 *       <DataTable.Col source="phone" />
 *       <DataTable.Col source="website" />
 *       <DataTable.Col source="company.name" />
 *     </DataTable>
 *   </List>
 * );
 */
function List<RecordType extends RaRecord = RaRecord>(
  props: ListProps<RecordType>,
) {
  const {
    debounce,
    disableAuthentication,
    disableSyncWithLocation,
    exporter,
    filter,
    filterDefaultValues,
    loading,
    perPage,
    queryOptions,
    resource,
    sort,
    storeKey,
    ...rest
  } = props;

  return (
    <ListBase<RecordType>
      debounce={debounce}
      disableAuthentication={disableAuthentication}
      disableSyncWithLocation={disableSyncWithLocation}
      exporter={exporter}
      filter={filter}
      filterDefaultValues={filterDefaultValues}
      loading={loading}
      perPage={perPage}
      queryOptions={queryOptions}
      resource={resource}
      sort={sort}
      storeKey={storeKey}
    >
      <ListView<RecordType> {...rest} />
    </ListBase>
  );
}

interface ListProps<RecordType extends RaRecord = RaRecord>
  extends ListBaseProps<RecordType>,
    // ra-core's ListBaseProps and ListViewProps declare `children` with
    // non-identical types; omit the overlap so the two can be combined (base
    // wins). Surfaces only under a consumer's strict `tsc -b`, not our build.
    Omit<ListViewProps<RecordType>, keyof ListBaseProps<RecordType>> {}

/**
 * The view component for List pages with layout and UI.
 *
 * @internal
 */
function ListView<RecordType extends RaRecord = RaRecord>(
  props: ListViewProps<RecordType>,
) {
  const {
    aside,
    component: Content = "div",
    disableBreadcrumb,
    empty = defaultEmpty,
    filters,
    pagination = defaultPagination,
    title,
    children,
    actions,
  } = props;
  const translate = useTranslate();
  const resource = useResourceContext();
  if (!resource) {
    throw new Error(
      "The ListView component must be used within a ResourceContextProvider",
    );
  }
  const getResourceLabel = useGetResourceLabel();
  const resourceLabel = getResourceLabel(resource, 2);
  const finalTitle =
    title !== undefined
      ? title
      : translate("ra.page.list", {
          name: resourceLabel,
        });
  const { hasCreate } = useResourceDefinition({ resource });
  const hasDashboard = useHasDashboard();
  const { data, isPending, filterValues, total } = useListContext<RecordType>();

  const dataIsEmpty = !data || (data as RecordType[]).length === 0;
  const shouldRenderEmpty =
    !isPending &&
    (total === 0 || (total == null && dataIsEmpty)) &&
    !Object.keys(filterValues).length &&
    empty !== false;

  return (
    <>
      {!disableBreadcrumb && (
        <Breadcrumb>
          {hasDashboard && (
            <BreadcrumbItem>
              <Link to="/">
                <Translate i18nKey="ra.page.dashboard">Home</Translate>
              </Link>
            </BreadcrumbItem>
          )}
          <BreadcrumbPage>{resourceLabel}</BreadcrumbPage>
        </Breadcrumb>
      )}

      <FilterContext.Provider value={filters}>
        <div className="flex justify-between items-start flex-wrap gap-2 my-2">
          <h2 className="text-2xl font-bold tracking-tight mb-2">
            {finalTitle}
          </h2>
          {actions ?? (
            <div className="flex items-center gap-2">
              {filters && filters.length > 0 ? <FilterButton /> : null}
              {hasCreate ? <CreateButton /> : null}
              {<ExportButton />}
            </div>
          )}
        </div>
        <FilterForm />

        <div className={cn("flex", aside ? "gap-4" : undefined)}>
          <div className="flex-1 min-w-0">
            {shouldRenderEmpty ? (
              empty
            ) : (
              <Content className={cn("my-2", props.className)}>
                {children}
              </Content>
            )}
            {!shouldRenderEmpty && pagination}
          </div>
          {aside}
        </div>
      </FilterContext.Provider>
    </>
  );
}

const defaultPagination = <ListPagination />;
const defaultEmpty = <Empty />;

interface ListViewProps<RecordType extends RaRecord = RaRecord> {
  aside?: ReactNode;
  children?: ReactNode;
  component?: ElementType;
  disableBreadcrumb?: boolean;
  empty?: ReactNode | false;
  render?: (props: ListControllerResult<RecordType, Error>) => ReactNode;
  actions?: ReactElement | false;
  filters?: ReactNode[];
  pagination?: ReactNode;
  title?: ReactNode | string | false;
  className?: string;
}

export { List, ListView, type ListProps, type ListViewProps };
