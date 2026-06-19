import type { ReactNode } from "react";
import type {
  RaRecord,
  UseReferenceManyFieldControllerParams,
  ListControllerResult,
} from "shadmin-core";
import { ReferenceManyFieldBase, useListContext } from "shadmin-core";
import type { FieldProps } from "@/lib/field-types";
import { Offline } from "@/components/admin/feedback/offline";

const defaultOffline = <Offline />;

interface ReferenceManyFieldProps<
  RecordType extends RaRecord = RaRecord,
  ReferenceRecordType extends RaRecord = RaRecord,
> extends Omit<FieldProps<RecordType>, "source" | "record" | "empty">,
    UseReferenceManyFieldControllerParams<RecordType, ReferenceRecordType>,
    ReferenceManyFieldViewProps<ReferenceRecordType> {}

interface ReferenceManyFieldViewProps<
  ReferenceRecordType extends RaRecord = RaRecord,
> {
  children?: ReactNode;
  empty?: ReactNode;
  error?: ReactNode;
  loading?: ReactNode;
  /**
   * Component to display when offline and the request is pending or has stale placeholder data.
   */
  offline?: ReactNode;
  pagination?: ReactNode;
  render?: (props: ListControllerResult<ReferenceRecordType>) => ReactNode;
}

/**
 * Displays multiple related records that reference the current record via a foreign key.
 *
 * This field fetches records from a related resource where the foreign key points to the current record.
 * It provides a ListContext to its children, enabling list rendering with DataTable or custom components.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/reference-many-field ReferenceManyField documentation}
 *
 * @example
 * import { Show, ReferenceManyField, DataTable, DateField, RecordField } from '@/components/admin';
 *
 * const AuthorShow = () => (
 *   <Show>
 *     <div className="flex flex-col gap-4">
 *       <RecordField source="first_name" />
 *       <RecordField source="last_name" />
 *       <RecordField label="Books">
 *         <ReferenceManyField reference="books" target="author_id" label="Books">
 *           <DataTable>
 *             <DataTable.Col source="title" />
 *             <DataTable.Col source="published_at" field={DateField} />
 *           </DataTable>
 *         </ReferenceManyField>
 *       </RecordField>
 *     </div>
 *   </Show>
 * );
 */
function ReferenceManyField<
  RecordType extends RaRecord = RaRecord,
  ReferenceRecordType extends RaRecord = RaRecord,
>(props: ReferenceManyFieldProps<RecordType, ReferenceRecordType>) {
  const {
    children,
    empty,
    error,
    loading,
    offline = defaultOffline,
    pagination,
    render,
    ...rest
  } = props;

  return (
    <ReferenceManyFieldBase {...rest} offline={offline}>
      <ReferenceManyFieldView<ReferenceRecordType>
        empty={empty}
        error={error}
        loading={loading}
        offline={offline}
        pagination={pagination}
        render={render}
      >
        {children}
      </ReferenceManyFieldView>
    </ReferenceManyFieldBase>
  );
}

function ReferenceManyFieldView<
  ReferenceRecordType extends RaRecord = RaRecord,
>(props: ReferenceManyFieldViewProps<ReferenceRecordType>) {
  const {
    children,
    empty,
    error: errorElement,
    loading,
    offline = defaultOffline,
    pagination,
    render,
  } = props;
  const listContext = useListContext();
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
  } = listContext;

  if (
    isPaused &&
    (isPending || isPlaceholderData) &&
    offline !== false &&
    offline !== undefined
  ) {
    return offline;
  }
  if (isPending && loading !== false) {
    return loading;
  }
  if (error && errorElement !== false) {
    return errorElement;
  }
  if (
    (total === 0 ||
      (total == null &&
        hasPreviousPage === false &&
        hasNextPage === false &&
        // @ts-expect-error FIXME total may be undefined when using partial pagination but the ListControllerResult type is wrong about it
        data.length === 0 &&
        // the user didn't set any filters
        !Object.keys(filterValues).length)) &&
    empty !== false
  ) {
    return empty;
  }

  return (
    <>
      {render?.(listContext)}
      {children}
      {pagination}
    </>
  );
}

export {
  ReferenceManyField,
  type ReferenceManyFieldProps,
  type ReferenceManyFieldViewProps,
};
