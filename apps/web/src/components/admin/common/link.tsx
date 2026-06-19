import { type Ref } from "react";
import {
  Link as RouterLink,
  type LinkProps as RouterLinkProps,
} from "react-router";
import { cn } from "@/lib/utils";

interface LinkProps extends RouterLinkProps {
  className?: string;
  ref?: Ref<HTMLAnchorElement>;
}

/**
 * Styled wrapper around `react-router`'s `<Link>` for use inside admin views.
 *
 * Mirrors the upstream `ra-ui-materialui` `<Link>` component: it forwards
 * its ref, accepts every prop of `react-router`'s `Link` and applies the
 * default underline-on-hover treatment so consumers don't need to repeat
 * Tailwind utility classes for every navigation link.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/link Link documentation}
 *
 * @example
 * import { Link } from "@/components/admin";
 *
 * <Link to="/posts/1/show">Show</Link>
 */
function Link({ className, ref, ...rest }: LinkProps) {
  return (
    <RouterLink
      ref={ref}
      className={cn(
        "text-primary underline-offset-4 hover:underline",
        className,
      )}
      {...rest}
    />
  );
}

export { Link, type LinkProps };
