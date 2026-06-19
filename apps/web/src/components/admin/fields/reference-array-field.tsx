import type { ReactElement, ReactNode } from "react";
import { memo } from "react";
import type {
  FilterPayload,
  ListControllerResult,
  RaRecord,
  SortPayload,
  ExtractRecordPaths,
  HintedString,
} from "shadmin-core";
import { ReferenceArrayFieldBase, useListContext } from "shadmin-core";
import type { UseQueryOptions } from "@tanstack/react-query";
import { LinearProgress } from "@/components/admin/feedback/linear-progress";
import { SingleFieldList } from "@/components/admin/list/single-field-list";
import type { FieldProps } from "@/lib/field-types";
import { Offline } from "@/components/admin/feedback/offline";

const defaultOffline = <Offline />;

interface ReferenceArrayFieldProps<
  RecordType extends RaRecord = RaRecord,
  ReferenceRecordType extends RaRecord = RaRecord,
> extends Omit<FieldProps<RecordType>, "source" | "empty">,
    ReferenceArrayFieldViewProps {
  filter?: FilterPayload;
  page?: number;
  pagination?: ReactElement;
  perPage?: number;
  reference: string;
  resource?: string;
  source: NoInfer<HintedString<ExtractRecordPaths<RecordType>>>;
  sort?: SortPayload;
  queryOptions?: Omit<
    UseQueryOptions<ReferenceRecordType[], Error>,
    "queryFn" | "queryKey"
  >;
  render?: (props: ListControllerResult<ReferenceRecordType>) => ReactElement;
}

interface ReferenceArrayFieldViewProps {
  children?: ReactNode;
  className?: string;
  empty?: ReactNode;
  error?: ReactNode;
  loading?: ReactNode;
  /**
   * Component to display when offline and the request is pending or has stale placeholder data.
   */
  offline?: ReactNode;
  pagination?: ReactNode;
}

/**
 * Displays multiple related records by following an array of foreign keys.
 *
 * This field fetches related records using an array of IDs and renders them using a child component (SingleFieldList by default).
 * It supports custom sorting, filtering, and rendering with DataTable or other list components.
 * To be used with RecordField or DataTable.Col components, or anywhere a RecordContext is available.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/reference-array-field ReferenceArrayField documentation}
 *
 * @example
 * import { List, DataTable, ReferenceArrayField } from '@/components/admin';
 *
 * export const PostList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col source="id" />
 *       <DataTable.Col source="title" />
 *       <DataTable.Col source="tag_ids" label="Tags">
 *         <ReferenceArrayField reference="tags" source="tag_ids" />
 *       </DataTable.Col>
 *       <DataTable.Col>
 *         <EditButton />
 *       </DataTable.Col>
 *     </DataTable>
 *   </List>
 * );
 */
function ReferenceArrayField<
  RecordType extends RaRecord = RaRecord,
  ReferenceRecordType extends RaRecord = RaRecord,
>(props: ReferenceArrayFieldProps<RecordType, ReferenceRecordType>) {
  const {
    empty,
    error,
    filter,
    offline = defaultOffline,
    page = 1,
    perPage,
    reference,
    resource,
    sort,
    source,
    queryOptions,
    render,
    ...rest
  } = props;
  return (
    <ReferenceArrayFieldBase
      filter={filter}
      offline={offline}
      page={page}
      perPage={perPage}
      reference={reference}
      resource={resource}
      sort={sort}
      source={source}
      // @ts-ignore ra-core/react-query type variance
      queryOptions={queryOptions}
      render={render}
    >
      <PureReferenceArrayFieldView
        empty={empty}
        error={error}
        offline={offline}
        {...rest}
      />
    </ReferenceArrayFieldBase>
  );
}

function ReferenceArrayFieldView(props: ReferenceArrayFieldViewProps) {
  const {
    children = defaultChildren,
    className,
    empty,
    error: errorElement,
    loading = defaultLoading,
    offline = defaultOffline,
    pagination,
  } = props;
  const {
    isPending,
    isPaused,
    isPlaceholderData,
    error,
    total,
    hasPreviousPage,
    hasNextPage,
    data,
    filterValues,
  } = useListContext();

  const shouldRenderOffline =
    isPaused &&
    (isPending || isPlaceholderData) &&
    offline !== undefined &&
    offline !== false;

  return (
    <div className={className}>
      {shouldRenderOffline ? (
        offline
      ) : isPending && loading !== false ? (
        loading
      ) : error && errorElement !== false ? (
        errorElement
      ) : (total === 0 ||
          (total == null &&
            hasPreviousPage === false &&
            hasNextPage === false &&
            // @ts-expect-error FIXME total may be undefined when using partial pagination but the ListControllerResult type is wrong about it
            data.length === 0 &&
            // the user didn't set any filters
            !Object.keys(filterValues).length)) &&
        empty !== false ? (
        empty
      ) : (
        <span>
          {children}
          {pagination && total !== undefined ? pagination : null}
        </span>
      )}
    </div>
  );
}

const defaultChildren = <SingleFieldList />;
const defaultLoading = <LinearProgress />;
const PureReferenceArrayFieldView = memo(ReferenceArrayFieldView);

export {
  ReferenceArrayField,
  ReferenceArrayFieldView,
  type ReferenceArrayFieldProps,
  type ReferenceArrayFieldViewProps,
};
