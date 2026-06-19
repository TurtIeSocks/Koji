import type { EditBaseProps } from "shadmin-core";
import {
  EditBase,
  Translate,
  useCreatePath,
  useEditContext,
  useGetRecordRepresentation,
  useGetResourceLabel,
  useHasDashboard,
  useResourceContext,
  useResourceDefinition,
} from "shadmin-core";
import type { ElementType, ReactNode } from "react";
import { Link } from "react-router";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbPage,
} from "@/components/admin/layout/breadcrumb";
import { cn } from "@/lib/utils";
import { ShowButton } from "@/components/admin/buttons/show-button";
import { DeleteButton } from "@/components/admin/buttons/delete-button";

// ra-core's EditViewProps and EditBaseProps declare `children` non-identically;
// omit the overlap so they combine (view wins). Surfaces only under a
// consumer's strict `tsc -b`, not our build.
interface EditProps
  extends EditBaseProps,
    Omit<EditViewProps, keyof EditBaseProps> {}

/**
 * A complete edit page with breadcrumb, title, and default actions.
 *
 * Combines data fetching, form context, and UI layout for editing records. Renders breadcrumb,
 * page title, Show and Delete buttons, and wraps your form components.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/edit Edit documentation}
 *
 * @example
 * import { Edit, SimpleForm, BooleanInput, TextInput } from "@/components/admin";
 * import { required } from 'ra-core';
 *
 * export const CustomerEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="first_name" validate={required()} />
 *       <TextInput source="last_name" validate={required()} />
 *       <TextInput source="email" validate={required()} />
 *       <BooleanInput source="has_ordered" />
 *       <TextInput multiline source="notes" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function Edit({
  actions,
  aside,
  children,
  className,
  component,
  disableBreadcrumb,
  title,
  ...rest
}: EditProps) {
  return (
    <EditBase {...rest}>
      <EditView
        actions={actions}
        aside={aside}
        className={className}
        component={component}
        disableBreadcrumb={disableBreadcrumb}
        title={title}
      >
        {children}
      </EditView>
    </EditBase>
  );
}

interface EditViewProps {
  aside?: ReactNode;
  component?: ElementType;
  disableBreadcrumb?: boolean;
  title?: ReactNode | string | false;
  actions?: ReactNode | false;
  children?: ReactNode;
  className?: string;
}

/**
 * The view component for Edit pages with layout and UI.
 *
 * @internal
 */
function EditView({
  aside,
  component,
  disableBreadcrumb,
  title,
  actions,
  className,
  children,
}: EditViewProps) {
  const context = useEditContext();

  const resource = useResourceContext();
  if (!resource) {
    throw new Error(
      "The EditView component must be used within a ResourceContextProvider",
    );
  }
  const getResourceLabel = useGetResourceLabel();
  const listLabel = getResourceLabel(resource, 2);
  const createPath = useCreatePath();
  const listLink = createPath({
    resource,
    type: "list",
  });

  const getRecordRepresentation = useGetRecordRepresentation(resource);
  const recordRepresentation = getRecordRepresentation(context.record);

  const { hasShow } = useResourceDefinition({ resource });
  const hasDashboard = useHasDashboard();

  const resolvedActions =
    actions === false
      ? null
      : (actions ?? (
          <div className="flex justify-end items-center gap-2">
            {hasShow ? <ShowButton /> : null}
            <DeleteButton />
          </div>
        ));

  const Wrapper = component ?? "div";

  const contentBlock = (
    <Wrapper className="my-2">{context.record ? children : null}</Wrapper>
  );

  const main = aside ? (
    <div className="flex gap-4">
      <div className="flex-1">{contentBlock}</div>
      <div className="flex-shrink-0 w-64">{aside}</div>
    </div>
  ) : (
    contentBlock
  );

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
          <BreadcrumbItem>
            <Link to={listLink}>{listLabel}</Link>
          </BreadcrumbItem>
          <BreadcrumbPage>{recordRepresentation}</BreadcrumbPage>
        </Breadcrumb>
      )}
      <div
        className={cn(
          "flex justify-between items-start flex-wrap gap-2 my-2",
          className,
        )}
      >
        {title !== false && (
          <h2 className="text-2xl font-bold tracking-tight">
            {title !== undefined ? title : context.defaultTitle}
          </h2>
        )}
        {resolvedActions}
      </div>
      {main}
    </>
  );
}

export { Edit, EditView, type EditProps, type EditViewProps };
