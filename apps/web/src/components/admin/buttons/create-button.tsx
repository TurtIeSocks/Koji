import React, { type Ref } from "react";
import { buttonVariants } from "@/components/ui/button";
import { Plus } from "lucide-react";
import {
  useCanAccess,
  useCreatePath,
  useGetResourceLabel,
  useResourceContext,
  useResourceTranslation,
} from "shadmin-core";
import { Link } from "react-router";

type CreateButtonProps = {
  label?: string;
  icon?: React.ReactNode;
  resource?: string;
  scrollToTop?: boolean;
  ref?: Ref<HTMLAnchorElement>;
};

const defaultIcon = <Plus />;

/**
 * A button that navigates to the create page for a resource.
 *
 * Automatically uses the current resource unless overridden.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/create-button CreateButton documentation}
 *
 * @example
 * import { CreateButton, List, ExportButton } from '@/components/admin';
 *
 * const PostList = () => (
 *   <List
 *     actions={<>
 *       <CreateButton />
 *       <ExportButton />
 *     </>}
 *   >
 *     ...
 *   </List>
 * );
 */
function CreateButton(props: CreateButtonProps) {
  const {
    label: labelProp,
    icon = defaultIcon,
    scrollToTop = true,
    ref,
  } = props;
  const resource = useResourceContext(props);
  const { canAccess, isPending } = useCanAccess({ action: "create", resource });
  const createPath = useCreatePath();
  const getResourceLabel = useGetResourceLabel();
  const link = createPath({
    resource,
    type: "create",
  });
  const label = useResourceTranslation({
    resourceI18nKey: resource
      ? `resources.${resource}.action.create`
      : undefined,
    baseI18nKey: "ra.action.create",
    options: {
      name: resource ? getResourceLabel(resource, 1) : undefined,
    },
    userText: labelProp,
  });
  if (isPending || !canAccess) return null;
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

export { CreateButton, type CreateButtonProps };
