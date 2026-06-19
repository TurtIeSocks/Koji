import type {
  InfiniteListBaseProps,
  InfiniteListControllerResult,
  RaRecord,
} from "shadmin-core";
import {
  FilterContext,
  InfiniteListBase,
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
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbPage,
} from "@/components/admin/layout/breadcrumb";
import { CreateButton } from "@/components/admin/buttons/create-button";
import { Empty } from "@/components/admin/feedback/empty";
import { ExportButton } from "@/components/admin/buttons/export-button";
import { FilterButton } from "@/components/admin/buttons/filter-button";
import { FilterForm } from "@/components/admin/list/filter-form";
import { InfinitePagination } from "@/components/admin/list/infinite-pagination";
import { cn } from "@/lib/utils";

/**
 * An infinite scroll variant of `<List>`, backed by `useInfiniteListController`.
 *
 * Renders the same breadcrumb / title / filter / actions chrome as `<List>` but
 * delegates pagination to `<InfinitePagination>`, which fetches the next page as
 * the user scrolls (or clicks the "Load more" button).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/infinite-list InfiniteList documentation}
 *
 * @example
 * import { DataTable, InfiniteList } from "@/components/admin";
 *
 * export const PostList = () => (
 *   <InfiniteList>
 *     <DataTable>
 *       <DataTable.Col source="id" />
 *       <DataTable.Col source="title" />
 *     </DataTable>
 *   </InfiniteList>
 * );
 */
function InfiniteList<RecordType extends RaRecord = RaRecord>(
  props: InfiniteListProps<RecordType>,
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
    <InfiniteListBase<RecordType>
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
      <InfiniteListView<RecordType> {...rest} />
    </InfiniteListBase>
  );
}

interface InfiniteListProps<RecordType extends RaRecord = RaRecord>
  extends InfiniteListBaseProps<RecordType>,
    // ra-core base/view props declare `children` non-identically; omit the
    // overlap so they combine (base wins). See list.tsx.
    Omit<
      InfiniteListViewProps<RecordType>,
      keyof InfiniteListBaseProps<RecordType>
    > {}

/**
 * The view component for `<InfiniteList>` pages with layout and UI.
 *
 * @internal
 */
function InfiniteListView<RecordType extends RaRecord = RaRecord>(
  props: InfiniteListViewProps<RecordType>,
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
      "The InfiniteListView component must be used within a ResourceContextProvider",
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
  const { data, isPending, filterValues, total, hasNextPage, hasPreviousPage } =
    useListContext<RecordType>();

  const dataIsEmpty = !data || (data as RecordType[]).length === 0;
  const shouldRenderEmpty =
    !isPending &&
    (total === 0 ||
      (total == null &&
        hasPreviousPage === false &&
        hasNextPage === false &&
        dataIsEmpty)) &&
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

const defaultPagination = <InfinitePagination />;
const defaultEmpty = <Empty />;

interface InfiniteListViewProps<RecordType extends RaRecord = RaRecord> {
  aside?: ReactNode;
  children?: ReactNode;
  component?: ElementType;
  disableBreadcrumb?: boolean;
  empty?: ReactNode | false;
  render?: (
    props: InfiniteListControllerResult<RecordType, Error>,
  ) => ReactNode;
  actions?: ReactElement | false;
  filters?: ReactNode[];
  pagination?: ReactNode;
  title?: ReactNode | string | false;
  className?: string;
}

export {
  InfiniteList,
  InfiniteListView,
  type InfiniteListProps,
  type InfiniteListViewProps,
};
