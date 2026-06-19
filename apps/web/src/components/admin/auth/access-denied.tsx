import type { HTMLAttributes, ReactNode } from "react";
import { Lock } from "lucide-react";

import { useTranslate } from "shadmin-core";
import { cn } from "@/lib/utils";

interface AccessDeniedProps extends HTMLAttributes<HTMLDivElement> {
  className?: string;
  icon?: ReactNode;
  textPrimary?: string;
  textSecondary?: string;
}

/**
 * Full-page error screen displayed when the current user does not have
 * permission to access a resource.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/access-denied AccessDenied documentation}
 */
function AccessDenied({
  icon = <Lock className="size-32" />,
  textPrimary = "ra.page.access_denied",
  textSecondary = "ra.message.access_denied",
  className,
  ...rest
}: AccessDeniedProps) {
  const translate = useTranslate();
  return (
    <div
      className={cn(
        "flex flex-col justify-center items-center h-full",
        className,
      )}
      {...rest}
    >
      <div className="flex flex-col items-center text-center pt-4 pb-4 opacity-50">
        {icon}
        <h5 className="text-secondary-foreground text-2xl mt-3">
          {translate(textPrimary, { _: textPrimary })}
        </h5>
        <p className="text-sm">
          {translate(textSecondary, { _: textSecondary })}
        </p>
      </div>
    </div>
  );
}

export { AccessDenied, type AccessDeniedProps };
