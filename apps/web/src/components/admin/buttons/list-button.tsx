import * as React from "react";
import type { Ref } from "react";
import { buttonVariants } from "@/components/ui/button";
import { List as ListIcon } from "lucide-react";
import {
  useCanAccess,
  useCreatePath,
  useGetResourceLabel,
  useResourceContext,
  useResourceTranslation,
} from "shadmin-core";
import { Link } from "react-router";
import { cn } from "@/lib/utils";

type ListButtonProps = {
  resource?: string;
  label?: string;
  icon?: React.ReactNode;
  className?: string;
  scrollToTop?: boolean;
  ref?: Ref<HTMLAnchorElement>;
};

/**
 * Opens the List view of a given resource.
 *
 * Reads the resource from `ResourceContext` by default. Commonly used in the actions of an
 * `<Edit>` or `<Show>` view to navigate back to the list.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/list-button ListButton documentation}
 *
 * @example
 * import { Edit, ListButton } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit actions={<ListButton />}>
 *     ...
 *   </Edit>
 * );
 */
function ListButton(props: ListButtonProps) {
  const {
    label: labelProp,
    icon = defaultIcon,
    className,
    scrollToTop = true,
    ref,
  } = props;
  const resource = useResourceContext(props);
  if (!resource) {
    throw new Error(
      "<ListButton> components should be used inside a <Resource> component or provided the resource prop.",
    );
  }
  const { canAccess, isPending } = useCanAccess({
    action: "list",
    resource,
  });
  const createPath = useCreatePath();
  const getResourceLabel = useGetResourceLabel();
  const label = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.action.list`,
    baseI18nKey: "ra.action.list",
    options: {
      name: getResourceLabel(resource, 1),
    },
    userText: labelProp,
  });

  if (!canAccess || isPending) {
    return null;
  }

  return (
    <Link
      ref={ref}
      className={cn(buttonVariants({ variant: "outline" }), className)}
      to={createPath({ type: "list", resource })}
      state={scrollToTop ? scrollState : undefined}
      aria-label={typeof label === "string" ? label : undefined}
    >
      {icon}
      {label}
    </Link>
  );
}

const defaultIcon = <ListIcon />;
const scrollState = { _scrollToTop: true };

export { ListButton, type ListButtonProps };
