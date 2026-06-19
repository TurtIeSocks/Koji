import type { Identifier, RaRecord } from "shadmin-core";
import {
  useCreatePath,
  useRecordContext,
  useResourceContext,
} from "shadmin-core";
import type { ReactNode } from "react";
import { Link } from "react-router";
import { cn } from "@/lib/utils";

/**
 * A function returning a React node from a record (and its identifier).
 *
 * Used by `<SimpleList>` and `<SimpleListItem>` for their slot props such as
 * `primaryText`, `secondaryText`, `leftAvatar`, etc.
 */
type FunctionToElement<RecordType extends RaRecord = RaRecord> = (
  record: RecordType,
  id: Identifier,
) => ReactNode;

interface SimpleListItemProps<RecordType extends RaRecord = RaRecord> {
  /**
   * The record to display. If omitted, the item reads it from the current
   * `<RecordContext>`.
   */
  record?: RecordType;
  /**
   * The resource the record belongs to. If omitted, the item reads it from
   * the current `<ResourceContext>`.
   */
  resource?: string;
  /**
   * Where to link when the user clicks the item. Defaults to `"edit"`, or
   * `"show"` when the resource has no edit view. Pass `false` to disable
   * the link entirely.
   */
  linkType?: "edit" | "show" | false;
  /**
   * Content rendered inside the item. Typically the inner row markup
   * produced by `<SimpleList>`.
   */
  children: ReactNode;
  /**
   * Optional zero-based index of the row inside its list.
   */
  rowIndex?: number;
  className?: string;
}

/**
 * A single row used by `<SimpleList>`, also usable on its own for manual
 * composition of record-driven list rows.
 *
 * When `linkType` is not `false` and a resource context is available,
 * the item is wrapped in a `<Link>` to the edit (default) or show page
 * of the record.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/simple-list-item SimpleListItem documentation}
 *
 * @example
 * import { SimpleListItem } from "@/components/admin";
 *
 * const PostRow = ({ record }: { record: Post }) => (
 *   <SimpleListItem record={record} linkType="show">
 *     <span className="font-medium">{record.title}</span>
 *   </SimpleListItem>
 * );
 */
function SimpleListItem<RecordType extends RaRecord = RaRecord>(
  props: SimpleListItemProps<RecordType>,
) {
  const { children, linkType, className } = props;
  const record = useRecordContext<RecordType>(props);
  const resource = useResourceContext(props);
  const createPath = useCreatePath();

  if (!record) return null;

  const rowClasses = cn(
    "flex items-center gap-3 px-3 py-2 hover:bg-accent rounded-md",
    className,
  );

  if (linkType === false || !resource) {
    return <li className={rowClasses}>{children}</li>;
  }

  const to = createPath({
    resource,
    type: linkType ?? "edit",
    id: record.id,
  });

  return (
    <li>
      <Link to={to} className={cn("block", rowClasses)}>
        {children}
      </Link>
    </li>
  );
}

export { SimpleListItem, type SimpleListItemProps, type FunctionToElement };
