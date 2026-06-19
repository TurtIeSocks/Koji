import * as React from "react";
import type { ReactNode } from "react";
import { Children } from "react";
import type { FormProps } from "shadmin-core";
import { Form } from "shadmin-core";
import { cn } from "@/lib/utils";
import { CancelButton } from "@/components/admin/buttons/cancel-button";
import { SaveButton } from "@/components/admin/buttons/save-button";

/**
 * A simple form layout with vertical stacking, validation, and default toolbar.
 *
 * Automatically includes a toolbar with Cancel and Save buttons unless you provide a custom toolbar.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/simple-form SimpleForm documentation}
 *
 * @example
 * import { Create, SimpleForm, TextInput } from '@/components/admin';
 *
 * const PostCreate = () => (
 *   <Create>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <TextInput source="body" />
 *     </SimpleForm>
 *   </Create>
 * );
 */
function SimpleForm({
  children,
  className,
  toolbar = defaultFormToolbar,
  ...rest
}: SimpleFormProps) {
  return (
    <Form
      className={cn(`flex flex-col gap-4 w-full max-w-lg`, className)}
      {...rest}
    >
      {children}
      {toolbar}
    </Form>
  );
}

/**
 * A sticky form toolbar with default Cancel and Save buttons.
 *
 * Provides a consistent action bar for forms that sticks to the bottom of the viewport. By default,
 * renders Cancel and Save buttons, but you can provide custom buttons as children.
 *
 * @example
 * import { FormToolbar, CancelButton, SaveButton } from '@/components/admin';
 *
 * const CustomToolbar = () => (
 *     <FormToolbar>
 *         <CancelButton />
 *         <SaveButton label="Publish" />
 *     </FormToolbar>
 * );
 */
function FormToolbar({ children, className, ...rest }: FormToolbarProps) {
  return (
    <div
      {...rest}
      className={cn(
        "sticky pt-4 pb-4 md:block md:pt-2 md:pb-0 bottom-0 bg-linear-to-b from-transparent to-background to-10%",
        className,
      )}
      role="toolbar"
    >
      {Children.count(children) === 0 ? (
        <div className="flex flex-row gap-2 justify-end">
          <CancelButton />
          <SaveButton />
        </div>
      ) : (
        children
      )}
    </div>
  );
}

type SimpleFormProps = {
  children: ReactNode;
  className?: string;
  toolbar?: ReactNode;
} & FormProps;

interface FormToolbarProps extends React.HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
  className?: string;
}

const defaultFormToolbar = <FormToolbar />;

export { SimpleForm, type SimpleFormProps, FormToolbar, type FormToolbarProps };
