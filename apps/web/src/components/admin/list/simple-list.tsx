import type { RaRecord } from "shadmin-core";
import {
  RecordContextProvider,
  useListContext,
  useResourceContext,
} from "shadmin-core";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import { ListNoResults } from "@/components/admin/list/list-no-results";
import {
  SimpleListItem,
  type FunctionToElement,
} from "@/components/admin/list/simple-list-item";
import { SimpleListLoading } from "@/components/admin/list/simple-list-loading";

interface SimpleListProps<RecordType extends RaRecord = RaRecord> {
  /**
   * A function returning the primary text for each record.
   */
  primaryText?: FunctionToElement<RecordType>;
  /**
   * A function returning the secondary text (rendered under the primary text).
   */
  secondaryText?: FunctionToElement<RecordType>;
  /**
   * A function returning the tertiary text (rendered floated to the right of
   * the primary text, often used for timestamps).
   */
  tertiaryText?: FunctionToElement<RecordType>;
  /**
   * A function returning an avatar element rendered before the primary text.
   */
  leftAvatar?: FunctionToElement<RecordType>;
  /**
   * A function returning an icon element rendered before the primary text.
   */
  leftIcon?: FunctionToElement<RecordType>;
  /**
   * A function returning an icon element rendered at the right of the row.
   */
  rightIcon?: FunctionToElement<RecordType>;
  /**
   * Where to link when the user clicks a row. Defaults to `"edit"`.
   * Pass `false` to disable the link entirely.
   */
  linkType?: "edit" | "show" | false;
  /**
   * Component rendered when the list is empty. Defaults to `<ListNoResults>`.
   */
  empty?: ReactNode;
  /**
   * Component rendered while the list is loading. Defaults to
   * `<SimpleListLoading>` with rows shaped after the configured slots.
   */
  loading?: ReactNode;
  className?: string;
}

/**
 * Render a list of records as a simple, mobile-friendly list (typically inside a `<List>`).
 *
 * Each row uses a set of render-prop slots (`primaryText`, `secondaryText`, `tertiaryText`,
 * `leftAvatar`, `leftIcon`, `rightIcon`) and links to the edit (default) or show page of
 * the record. Pass `linkType={false}` to disable the link.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/simple-list SimpleList documentation}
 *
 * @example
 * import { List, SimpleList } from "@/components/admin";
 *
 * export const PostList = () => (
 *   <List>
 *     <SimpleList
 *       primaryText={(record) => record.title}
 *       secondaryText={(record) => `${record.views} views`}
 *       tertiaryText={(record) =>
 *         new Date(record.published_at).toLocaleDateString()
 *       }
 *     />
 *   </List>
 * );
 */
function SimpleList<RecordType extends RaRecord = RaRecord>(
  props: SimpleListProps<RecordType>,
) {
  const {
    primaryText,
    secondaryText,
    tertiaryText,
    leftAvatar,
    leftIcon,
    rightIcon,
    linkType,
    empty = defaultEmpty,
    loading,
    className,
  } = props;

  const { data, isPending } = useListContext<RecordType>();
  const resource = useResourceContext();

  if (isPending) {
    return (
      loading ?? (
        <SimpleListLoading
          className={className}
          hasLeftAvatarOrIcon={!!leftIcon || !!leftAvatar}
          hasRightAvatarOrIcon={!!rightIcon}
          hasSecondaryText={!!secondaryText}
          hasTertiaryText={!!tertiaryText}
        />
      )
    );
  }

  if (!data || data.length === 0) return <>{empty}</>;

  return (
    <ul className={cn("flex flex-col", className)}>
      {data.map((record) => (
        <RecordContextProvider key={record.id} value={record}>
          <SimpleListItem<RecordType>
            record={record}
            resource={resource}
            linkType={linkType}
          >
            {(leftIcon || leftAvatar) && (
              <div className="shrink-0">
                {leftAvatar ? leftAvatar(record, record.id) : null}
                {leftIcon ? leftIcon(record, record.id) : null}
              </div>
            )}
            <div className="flex-1 min-w-0">
              <div className="flex items-baseline justify-between gap-2">
                {primaryText && (
                  <div className="font-medium truncate">
                    {primaryText(record, record.id)}
                  </div>
                )}
                {tertiaryText && (
                  <div className="text-xs text-muted-foreground/70 shrink-0">
                    {tertiaryText(record, record.id)}
                  </div>
                )}
              </div>
              {secondaryText && (
                <div className="text-sm text-muted-foreground truncate">
                  {secondaryText(record, record.id)}
                </div>
              )}
            </div>
            {rightIcon && (
              <div className="shrink-0">{rightIcon(record, record.id)}</div>
            )}
          </SimpleListItem>
        </RecordContextProvider>
      ))}
    </ul>
  );
}

const defaultEmpty = <ListNoResults />;

export { SimpleList, type SimpleListProps };
