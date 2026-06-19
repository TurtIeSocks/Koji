import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbPage,
} from "@/components/admin/layout/breadcrumb";
import type { CreateBaseProps } from "shadmin-core";
import {
  CreateBase,
  Translate,
  useCreateContext,
  useCreatePath,
  useGetResourceLabel,
  useHasDashboard,
  useResourceContext,
} from "shadmin-core";
import type { ElementType, ReactNode } from "react";
import { Link } from "react-router";
import { cn } from "@/lib/utils";

type CreateProps = CreateViewProps & CreateBaseProps;

/**
 * A complete create page with breadcrumb, title, and actions.
 *
 * Combines data fetching, form context, and UI layout for creating new records. Renders breadcrumb
 * navigation, page title, and wraps your form components.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/create Create documentation}
 *
 * @example
 * import { Create, SimpleForm, TextInput } from '@/components/admin';
 *
 * export const PostCreate = () => (
 *   <Create>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <TextInput source="body" />
 *     </SimpleForm>
 *   </Create>
 * );
 */
function Create({
  actions,
  aside,
  children,
  className,
  component,
  disableBreadcrumb,
  title,
  ...rest
}: CreateProps) {
  return (
    <CreateBase {...rest}>
      <CreateView
        actions={actions}
        aside={aside}
        className={className}
        component={component}
        disableBreadcrumb={disableBreadcrumb}
        title={title}
      >
        {children}
      </CreateView>
    </CreateBase>
  );
}

type CreateViewProps = {
  actions?: ReactNode;
  aside?: ReactNode;
  component?: ElementType;
  disableBreadcrumb?: boolean;
  children?: ReactNode;
  className?: string;
  title?: ReactNode | string | false;
};

/**
 * The view component for Create pages with layout and UI.
 *
 * @internal
 */
function CreateView({
  actions,
  aside,
  component,
  disableBreadcrumb,
  title,
  children,
  className,
}: CreateViewProps) {
  const context = useCreateContext();

  const resource = useResourceContext();
  if (!resource) {
    throw new Error(
      "The CreateView component must be used within a ResourceContextProvider",
    );
  }
  const getResourceLabel = useGetResourceLabel();
  const listLabel = getResourceLabel(resource, 2);
  const createPath = useCreatePath();
  const listLink = createPath({
    resource,
    type: "list",
  });
  const hasDashboard = useHasDashboard();

  const Wrapper = component ?? "div";

  const header = (
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
      {actions}
    </div>
  );

  const contentBlock = <Wrapper className="my-2">{children}</Wrapper>;

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
          <BreadcrumbPage>
            <Translate i18nKey="ra.action.create">Create</Translate>
          </BreadcrumbPage>
        </Breadcrumb>
      )}
      {header}
      {main}
    </>
  );
}

export { Create, CreateView, type CreateProps, type CreateViewProps };
