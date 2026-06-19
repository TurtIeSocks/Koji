import React, { type Ref } from "react";
import { Link } from "react-router";
import { buttonVariants } from "@/components/ui/button";
import { Eye } from "lucide-react";
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

type ShowButtonProps = {
  label?: string;
  icon?: React.ReactNode;
  record?: RaRecord;
  resource?: string;
  scrollToTop?: boolean;
  ref?: Ref<HTMLAnchorElement>;
} & React.AnchorHTMLAttributes<HTMLAnchorElement>;

/**
 * A button that navigates to the show page for a record.
 *
 * Works within RecordContext to automatically get the record ID.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/show-button ShowButton documentation}
 *
 * @example
 * import { ShowButton } from '@/components/admin';
 *
 * const PostActions = () => (
 *   <ShowButton label="View Details" />
 * );
 */
function ShowButton(props: ShowButtonProps) {
  const {
    label: labelProp,
    icon,
    record: _record,
    resource: _resource,
    scrollToTop = true,
    ref,
    ...rest
  } = props;
  const resource = useResourceContext(props);
  const record = useRecordContext(props);
  const createPath = useCreatePath();
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "show",
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
    resourceI18nKey: resource ? `resources.${resource}.action.show` : undefined,
    baseI18nKey: "ra.action.show",
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
    type: "show",
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
      {...rest}
    >
      {icon ?? <Eye />}
      {label}
    </Link>
  );
}

// useful to prevent click bubbling in a datagrid with rowClick
const stopPropagation = (e: React.MouseEvent) => e.stopPropagation();

export { ShowButton, type ShowButtonProps };
