import React, { type Ref } from "react";
import { buttonVariants } from "@/components/ui/button";
import { Pencil } from "lucide-react";
import type { RaRecord } from "shadmin-core";
import {
  useCanAccess,
  useCreatePath,
  useGetRecordRepresentation,
  useGetResourceLabel,
  useRecordContext,
  useResourceContext,
  useResourceTranslation,
} from "shadmin-core";
import { Link } from "react-router";

type EditButtonProps = {
  record?: RaRecord;
  resource?: string;
  label?: string;
  icon?: React.ReactNode;
  scrollToTop?: boolean;
  ref?: Ref<HTMLAnchorElement>;
};

const defaultIcon = <Pencil />;

/**
 * A button that navigates to the edit page for a record.
 *
 * Works within RecordContext to automatically get the record ID.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/edit-button EditButton documentation}
 *
 * @example
 * import { DataTable, EditButton } from '@/components/admin';
 *
 * const PostList = () => (
 *   <DataTable>
 *     <DataTable.Col source="title" />
 *     <DataTable.Col source="author" />
 *     <DataTable.Col source="published_at" />
 *     <DataTable.Col>
 *       <EditButton />
 *     </DataTable.Col>
 *   </DataTable>
 * );
 */
function EditButton(props: EditButtonProps) {
  const {
    label: labelProp,
    icon = defaultIcon,
    scrollToTop = true,
    ref,
  } = props;
  const resource = useResourceContext(props);
  const record = useRecordContext(props);
  const createPath = useCreatePath();
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "edit",
    resource,
    record,
  });
  const getResourceLabel = useGetResourceLabel();
  const getRecordRepresentation = useGetRecordRepresentation(resource);
  const recordRepresentationValue = getRecordRepresentation(record);
  const recordRepresentation =
    typeof recordRepresentationValue === "string"
      ? recordRepresentationValue
      : recordRepresentationValue?.toString();
  const label = useResourceTranslation({
    resourceI18nKey: resource ? `resources.${resource}.action.edit` : undefined,
    baseI18nKey: "ra.action.edit",
    options: {
      name: resource ? getResourceLabel(resource, 1) : undefined,
      recordRepresentation,
    },
    userText: labelProp,
  });
  if (!record || record.id == null || !canAccess || isAccessPending) {
    return null;
  }
  const link = createPath({
    resource,
    type: "edit",
    id: record.id,
  });
  return (
    <Link
      ref={ref}
      className={buttonVariants({ variant: "outline" })}
      to={link}
      state={{ _scrollToTop: scrollToTop }}
      onClick={stopPropagation}
      aria-label={typeof label === "string" ? label : undefined}
    >
      {icon}
      {label}
    </Link>
  );
}

// useful to prevent click bubbling in a datagrid with rowClick
const stopPropagation = (e: React.MouseEvent) => e.stopPropagation();

export { EditButton, type EditButtonProps };
