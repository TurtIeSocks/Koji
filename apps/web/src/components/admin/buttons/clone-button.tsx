import * as React from "react";
import type { Ref } from "react";
import { buttonVariants } from "@/components/ui/button";
import { Copy } from "lucide-react";
import type { RaRecord } from "shadmin-core";
import {
  useCreatePath,
  useGetResourceLabel,
  useRecordContext,
  useResourceContext,
  useResourceTranslation,
} from "shadmin-core";
import { Link } from "react-router";
import { cn } from "@/lib/utils";

type CloneButtonProps = {
  record?: Partial<RaRecord>;
  resource?: string;
  label?: string;
  icon?: React.ReactNode;
  className?: string;
  scrollToTop?: boolean;
  ref?: Ref<HTMLAnchorElement>;
};

/**
 * A button that navigates to the create page, pre-filled with the current record's data.
 *
 * Used to clone the current record. Reads the resource from `ResourceContext` and the record
 * from `RecordContext` by default.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/clone-button CloneButton documentation}
 *
 * @example
 * import { CloneButton, Show } from '@/components/admin';
 *
 * const PostShow = () => (
 *   <Show actions={<CloneButton />}>
 *     ...
 *   </Show>
 * );
 */
function CloneButton(props: CloneButtonProps) {
  const {
    label: labelProp,
    icon = defaultIcon,
    className,
    scrollToTop = true,
    ref,
  } = props;
  const resource = useResourceContext(props);
  const record = useRecordContext(props);
  const createPath = useCreatePath();
  const getResourceLabel = useGetResourceLabel();
  const label = useResourceTranslation({
    resourceI18nKey: resource
      ? `resources.${resource}.action.clone`
      : undefined,
    baseI18nKey: "ra.action.clone",
    options: {
      name: resource ? getResourceLabel(resource, 1) : undefined,
    },
    userText: labelProp,
  });
  if (!record || record.id == null) return null;
  const pathname = createPath({ resource, type: "create" });
  const recordWithoutId = omitId(record);
  return (
    <Link
      ref={ref}
      className={cn(buttonVariants({ variant: "outline" }), className)}
      to={{
        pathname,
        search: recordWithoutId
          ? `?source=${encodeURIComponent(JSON.stringify(recordWithoutId))}`
          : undefined,
      }}
      state={{ _scrollToTop: scrollToTop }}
      onClick={stopPropagation}
      aria-label={typeof label === "string" ? label : undefined}
    >
      {icon}
      {label}
    </Link>
  );
}

const defaultIcon = <Copy />;

// useful to prevent click bubbling in a datagrid with rowClick
const stopPropagation = (e: React.MouseEvent) => e.stopPropagation();

const omitId = (record: Partial<RaRecord>) => {
  const { id: _id, ...rest } = record;
  return rest;
};

export { CloneButton, type CloneButtonProps };
