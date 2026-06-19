import type { HTMLAttributes } from "react";
import { useTimeout } from "shadmin-core";

import { cn } from "@/lib/utils";

interface LinearProgressProps extends HTMLAttributes<HTMLDivElement> {
  /**
   * Delay in milliseconds before showing the progress bar.
   * @default 1000
   */
  timeout?: number;
  /**
   * When provided, switches to determinate mode and renders the bar at
   * `(value / max) * 100` percent width.
   */
  value?: number;
  /**
   * Maximum value used to compute the fill percentage.
   * @default 100
   */
  max?: number;
}

/**
 * Linear progress bar with a delay before it appears, designed to replace
 * an input or a field in a form layout while data is loading. Avoids
 * visual jumps when finally replaced by the resolved value.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/linear-progress LinearProgress documentation}
 *
 * @example
 * import { LinearProgress } from "@/components/admin/feedback/linear-progress";
 *
 * const ReferenceFieldFallback = () => <LinearProgress />;
 */
function LinearProgress(props: LinearProgressProps) {
  const { className, timeout = 1000, value, max = 100, ...rest } = props;
  const oneSecondHasPassed = useTimeout(timeout);

  if (!oneSecondHasPassed) {
    return <span className="block my-1 h-1" />;
  }

  return (
    <div
      className={cn(
        "my-2 w-40 h-1 overflow-hidden rounded bg-muted",
        className,
      )}
      {...rest}
    >
      {value !== undefined ? (
        <div
          className="h-full bg-primary transition-all"
          style={{ width: `${(value / max) * 100}%` }}
        />
      ) : (
        <div className="h-full w-1/2 bg-primary animate-pulse" />
      )}
    </div>
  );
}

export { LinearProgress, type LinearProgressProps };
