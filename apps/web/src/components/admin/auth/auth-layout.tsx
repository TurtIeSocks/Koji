import type { HTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib/utils";
import { Notification } from "@/components/admin/feedback/notification";

interface AuthLayoutProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  /**
   * Content rendered in the main pane — typically a login, signup or
   * password-reset form.
   */
  children: ReactNode;
  /**
   * Optional heading shown above the main-pane content.
   */
  title?: ReactNode;
  /**
   * Optional subtitle shown below the heading.
   */
  subtitle?: ReactNode;
  /**
   * Optional left-pane content (e.g. brand logo + testimonial). When
   * omitted, the layout renders the main pane full-width.
   */
  aside?: ReactNode;
  /**
   * Extra class names applied to the root wrapper.
   */
  className?: string;
}

/**
 * Page shell for custom authentication flows (login, signup, password reset).
 *
 * Renders a centered single-column form by default. Pass an `aside` to
 * opt into a two-column layout with custom brand / marketing content on
 * the left. Mounts the `<Notification>` component at the root so error
 * and info messages triggered with `useNotify()` are displayed.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/auth-layout AuthLayout documentation}
 */
function AuthLayout(props: AuthLayoutProps) {
  const { children, title, subtitle, aside, className, ...rest } = props;
  const hasAside = aside != null;
  return (
    <div className={cn("min-h-screen flex", className)} {...rest}>
      <div
        className={cn(
          "container relative grid flex-col items-center justify-center sm:max-w-none lg:px-0",
          hasAside && "lg:grid-cols-2",
        )}
      >
        {hasAside && (
          <div className="relative hidden h-full flex-col bg-zinc-900 p-10 text-white dark:border-r lg:flex">
            {aside}
          </div>
        )}
        <div className="lg:p-8">
          <div className="mx-auto flex w-full flex-col justify-center gap-y-6 sm:w-87.5">
            {(title || subtitle) && (
              <div className="flex flex-col gap-y-2 text-center">
                {title && (
                  <h1 className="text-2xl font-semibold tracking-tight">
                    {title}
                  </h1>
                )}
                {subtitle && (
                  <p className="text-sm leading-none text-muted-foreground">
                    {subtitle}
                  </p>
                )}
              </div>
            )}
            {children}
          </div>
        </div>
      </div>
      <Notification />
    </div>
  );
}

export { AuthLayout, type AuthLayoutProps };
