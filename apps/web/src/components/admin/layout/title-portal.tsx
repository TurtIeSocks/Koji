import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";
import { TITLE_PORTAL_ID } from "@/lib/title-portal-id";

type TitlePortalProps = HTMLAttributes<HTMLDivElement>;

/**
 * Render target for `<Title>`, placed in the app bar.
 *
 * `<TitlePortal>` reserves the DOM slot where page titles (and breadcrumbs)
 * are teleported. The default Layout includes it inside `<AppBar>`, but it
 * can also be used directly when composing a custom header.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/title-portal TitlePortal documentation}
 *
 * @example
 * import { TitlePortal, Title } from "@/components/admin";
 *
 * const CustomHeader = () => (
 *   <header className="flex items-center px-4 h-12">
 *     <TitlePortal />
 *   </header>
 * );
 */
function TitlePortal({ className, ...rest }: TitlePortalProps) {
  return (
    <div
      id={TITLE_PORTAL_ID}
      className={cn("flex flex-1 items-center min-w-0", className)}
      {...rest}
    />
  );
}

export { TitlePortal, type TitlePortalProps };
