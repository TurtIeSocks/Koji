import type { ReactNode } from "react";
import { Children } from "react";
import { cn } from "@/lib/utils";
import { SaveButton } from "@/components/admin/buttons/save-button";
import { DeleteButton } from "@/components/admin/buttons/delete-button";

/**
 * The default toolbar rendered at the bottom of Edit and Create forms.
 *
 * When no children are provided it renders a `<SaveButton>` on the left and a
 * `<DeleteButton>` on the right — the standard edit-form layout. Pass children
 * to replace the default buttons with your own.
 *
 * Pass `resource` to forward it to the inner `<DeleteButton>` when the toolbar
 * is used outside a `ResourceContext`.
 *
 * @see {@link https://marmelab.com/react-admin/Toolbar.html}
 *
 * @example // Custom toolbar with only a SaveButton
 * import { Edit, SimpleForm, Toolbar, SaveButton } from '@/components/admin';
 *
 * const MyToolbar = () => (
 *   <Toolbar>
 *     <SaveButton />
 *   </Toolbar>
 * );
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm toolbar={<MyToolbar />}>…</SimpleForm>
 *   </Edit>
 * );
 *
 * @example // Always-enabled save button
 * const AlwaysSaveToolbar = () => (
 *   <Toolbar>
 *     <SaveButton alwaysEnable />
 *   </Toolbar>
 * );
 */
function Toolbar({ children, className, resource, ...rest }: ToolbarProps) {
  return (
    <div
      {...rest}
      role="toolbar"
      className={cn(
        "sticky pt-4 pb-4 md:block md:pt-2 md:pb-0 bottom-0 bg-linear-to-b from-transparent to-background to-10%",
        className,
      )}
    >
      {Children.count(children) === 0 ? (
        <div className="flex flex-row gap-2 justify-between">
          <SaveButton />
          <DeleteButton resource={resource} variant="ghost" />
        </div>
      ) : (
        children
      )}
    </div>
  );
}

interface ToolbarProps extends React.HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
  className?: string;
  /** Forward to the default `<DeleteButton>` when used outside a ResourceContext. */
  resource?: string;
}

export { Toolbar, type ToolbarProps };
