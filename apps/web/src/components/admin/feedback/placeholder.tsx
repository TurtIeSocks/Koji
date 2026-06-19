import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface PlaceholderProps {
  className?: string;
  children?: ReactNode;
}

/**
 * Tiny placeholder block used while data is loading.
 *
 * Renders an inline-flex span filled with the muted background colour so
 * a row of fields keeps its height during a fetch.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/placeholder Placeholder documentation}
 *
 * @example
 * import { Placeholder } from "@/components/admin/feedback/placeholder";
 *
 * const LoadingField = () => <Placeholder className="w-24" />;
 */
function Placeholder({ className, children }: PlaceholderProps) {
  return (
    <span className={cn("inline-flex bg-muted", className)}>
      {children ?? " "}
    </span>
  );
}

export { Placeholder, type PlaceholderProps };
