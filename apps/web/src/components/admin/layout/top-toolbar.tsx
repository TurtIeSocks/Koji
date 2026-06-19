import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/utils";

type TopToolbarProps = HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
};

/**
 * Container for action buttons positioned at the top right of a view.
 *
 * Convenience wrapper that lays out toolbar items horizontally with end
 * alignment and a small gap. Matches the `TopToolbar` from
 * `ra-ui-materialui` for API parity. Useful as the `actions` prop of
 * `<List>`, `<Show>`, `<Edit>`, etc.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/top-toolbar TopToolbar documentation}
 *
 * @example
 * import {
 *   CreateButton,
 *   ExportButton,
 *   List,
 *   TopToolbar,
 * } from "@/components/admin";
 *
 * const PostListActions = () => (
 *   <TopToolbar>
 *     <CreateButton />
 *     <ExportButton />
 *   </TopToolbar>
 * );
 *
 * export const PostList = () => (
 *   <List actions={<PostListActions />}>
 *     ...
 *   </List>
 * );
 */
function TopToolbar({ children, className, ...rest }: TopToolbarProps) {
  return (
    <div
      className={cn("flex items-center justify-end gap-2 mb-2", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export { TopToolbar, type TopToolbarProps };
