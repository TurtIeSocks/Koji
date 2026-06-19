import { Inbox } from "lucide-react";
import {
  useGetResourceLabel,
  useResourceContext,
  useResourceDefinition,
  useTranslate,
} from "shadmin-core";
import { cn } from "@/lib/utils";
import { CreateButton } from "@/components/admin/buttons/create-button";

interface EmptyProps {
  resource?: string;
  hasCreate?: boolean;
  className?: string;
}

/**
 * Default empty page rendered by `<List>` when the data provider returns
 * zero records and there are no active filters.
 *
 * Displays a centered illustration, an empty-state message and (when the
 * resource has a create route) an invitation with a `<CreateButton>`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/empty Empty documentation}
 *
 * @example
 * import { List, Empty } from "@/components/admin";
 *
 * export const PostList = () => (
 *   <List empty={<Empty />}>
 *     ...
 *   </List>
 * );
 */
function Empty(props: EmptyProps) {
  const { className } = props;
  const resource = useResourceContext(props);
  const { hasCreate: hasCreateFromDefinition } = useResourceDefinition(props);
  const hasCreate = props.hasCreate ?? hasCreateFromDefinition;
  const translate = useTranslate();
  const getResourceLabel = useGetResourceLabel();
  const resourceName = translate(`resources.${resource}.forcedCaseName`, {
    smart_count: 0,
    _: resource ? getResourceLabel(resource, 0) : undefined,
  });
  const emptyMessage = translate("ra.page.empty", { name: resourceName });
  const inviteMessage = translate("ra.page.invite");

  return (
    <div
      className={cn(
        "flex flex-1 flex-col items-center justify-center gap-2 py-16 text-center",
        className,
      )}
    >
      <Inbox className="size-16 text-muted-foreground" aria-hidden="true" />
      <h2 className="text-xl font-semibold mt-2">
        {translate(`resources.${resource}.empty`, { _: emptyMessage })}
      </h2>
      {hasCreate && (
        <>
          <p className="text-muted-foreground mb-2">
            {translate(`resources.${resource}.invite`, { _: inviteMessage })}
          </p>
          <CreateButton />
        </>
      )}
    </div>
  );
}

export { Empty, type EmptyProps };
