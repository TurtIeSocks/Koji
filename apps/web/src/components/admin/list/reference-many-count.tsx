import type { ReactNode } from "react";
import type { RaRecord, SortPayload } from "shadmin-core";
import type { UnknownRecord } from "@/lib/unknown-types";
import type { FieldProps } from "@/lib/field-types";
import {
  useCreatePath,
  useRecordContext,
  useReferenceManyFieldController,
  useTimeout,
  useTranslate,
} from "shadmin-core";
import { CircleX } from "lucide-react";
import get from "lodash/get";
import { Link } from "react-router";
import { Spinner } from "@/components/ui/spinner";
import { Offline } from "@/components/admin/feedback/offline";

const defaultOffline = <Offline />;

interface ReferenceManyCountProps<RecordType extends RaRecord = RaRecord>
  extends Omit<FieldProps<RecordType>, "source" | "record" | "empty"> {
  record?: RecordType;
  reference: string;
  resource?: string;
  target: string;
  source?: string;
  sort?: SortPayload;
  filter?: UnknownRecord;
  link?: boolean;
  /**
   * Component to display when offline and the request is pending or has stale placeholder data.
   */
  offline?: ReactNode;
  timeout?: number;
}

/**
 * Displays the count of related records that reference the current record.
 *
 * Calls dataProvider.getList() to compute the the number of records in a related resource that have a foreign key pointing to the current record.
 * It can optionally link to a filtered list of those records.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/reference-many-count ReferenceManyCount documentation}
 *
 * @example
 * import { List, DataTable, ReferenceManyCount } from '@/components/admin';
 *
 * const AuthorList = () => (
 *   <List>
 *     <DataTable>
 *       <DataTable.Col source="name" />
 *       <DataTable.Col label="Number of Books">
 *         <ReferenceManyCount reference="books" target="author_id" link />
 *       </DataTable.Col>
 *     </DataTable>
 *   </List>
 * );
 */
function ReferenceManyCount<RecordType extends RaRecord = RaRecord>(
  props: ReferenceManyCountProps<RecordType>,
) {
  const {
    reference,
    target,
    filter,
    sort,
    link,
    offline = defaultOffline,
    resource,
    source = "id",
    timeout = 1000,
  } = props;
  const record = useRecordContext<RecordType>(props);
  const createPath = useCreatePath();
  const translate = useTranslate();
  const timeoutReached = useTimeout(timeout);

  const { isPending, isPaused, error, total } =
    useReferenceManyFieldController<RecordType>({
      filter,
      sort,
      page: 1,
      perPage: 1,
      record,
      reference,
      resource,
      source,
      target,
    });

  const body =
    isPaused && isPending && offline !== undefined && offline !== false ? (
      offline
    ) : isPending ? (
      timeoutReached ? (
        <Spinner />
      ) : (
        ""
      )
    ) : error ? (
      <CircleX
        className="size-4 text-destructive"
        aria-label={translate("ra.notification.http_error", { _: "Error" })}
      />
    ) : (
      total
    );

  return link && record ? (
    <Link
      to={{
        pathname: createPath({ resource: reference, type: "list" }),
        search: `filter=${JSON.stringify({
          ...(filter || {}),
          [target]: get(record, source),
        })}`,
      }}
      onClick={(e) => e.stopPropagation()}
    >
      {body}
    </Link>
  ) : (
    <span>{body}</span>
  );
}

export { ReferenceManyCount, type ReferenceManyCountProps };
