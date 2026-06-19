import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbPage,
} from "@/components/admin/layout/breadcrumb";
import type { ShowBaseProps } from "shadmin-core";
import {
  ShowBase,
  Translate,
  useCreatePath,
  useHasDashboard,
  useShowContext,
  useGetRecordRepresentation,
  useGetResourceLabel,
  useResourceContext,
  useResourceDefinition,
} from "shadmin-core";
import type { ElementType, ReactNode } from "react";
import { Link } from "react-router";
import { cn } from "@/lib/utils";
import { EditButton } from "@/components/admin/buttons/edit-button";

// ra-core's ShowViewProps and ShowBaseProps declare `children` non-identically;
// omit the overlap so they combine (view wins). Surfaces only under a
// consumer's strict `tsc -b`, not our build.
interface ShowProps
  extends ShowBaseProps,
    Omit<ShowViewProps, keyof ShowBaseProps> {}

/**
 * A complete show page with breadcrumb, title, and default actions.
 *
 * Combines data fetching and UI layout for displaying record details. Inside, use
 * RecordField to display individual fields with labels.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/show Show documentation}
 *
 * @example
 * import { RecordField, NumberField, ReferenceField, Show } from "@/components/admin";
 *
 * export const ProductShow = () => (
 *   <Show>
 *     <div className="flex flex-col gap-4">
 *       <RecordField source="reference" />
 *       <RecordField source="category_id">
 *         <ReferenceField source="category_id" reference="categories" />
 *       </RecordField>
 *       <RecordField
 *         source="price"
 *         render={(record) => Intl.NumberFormat().format(record.price)}
 *       />
 *       <RecordField source="size" field={NumberField} />
 *     </div>
 *   </Show>
 * );
 */
function Show({
  actions,
  aside,
  children,
  className,
  component,
  disableBreadcrumb,
  title,
  ...rest
}: ShowProps) {
  return (
    <ShowBase {...rest}>
      <ShowView
        title={title}
        actions={actions}
        aside={aside}
        className={className}
        component={component}
        disableBreadcrumb={disableBreadcrumb}
      >
        {children}
      </ShowView>
    </ShowBase>
  );
}

interface ShowViewProps {
  actions?: ReactNode | false;
  aside?: ReactNode;
  component?: ElementType;
  disableBreadcrumb?: boolean;
  children?: ReactNode;
  className?: string;
  title?: ReactNode | string | false;
}

/**
 * The view component for Show pages with layout and UI.
 *
 * Renders breadcrumb, title, and default actions for show pages. Use Show instead unless you need
 * custom data fetching logic with ShowBase.
 *
 * @example
 * import { ShowBase, ShowView, SimpleShowLayout } from '@/components/admin';
 *
 * export const PostShow = () => (
 *     <ShowBase>
 *         <ShowView>
 *             <SimpleShowLayout>...</SimpleShowLayout>
 *         </ShowView>
 *     </ShowBase>
 * );
 */
function ShowView({
  actions,
  aside,
  children,
  className,
  component,
  disableBreadcrumb,
  title,
}: ShowViewProps) {
  const context = useShowContext();

  const resource = useResourceContext();
  if (!resource) {
    throw new Error(
      "The ShowView component must be used within a ResourceContextProvider",
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

  const { hasEdit } = useResourceDefinition({ resource });
  const hasDashboard = useHasDashboard();

  const resolvedActions =
    actions === false
      ? null
      : (actions ?? (
          <div className="flex justify-end items-center">
            {hasEdit ? <EditButton /> : null}
          </div>
        ));

  const Wrapper = component ?? "div";

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

export { Show, type ShowProps, ShowView, type ShowViewProps };
