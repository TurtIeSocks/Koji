import { cn } from "@/lib/utils";
import { Spinner as UISpinner } from "@/components/ui/spinner";

const sizeClasses = {
  small: "size-6",
  medium: "size-8",
  large: "size-12",
} as const;

interface SpinnerContentProps {
  size?: keyof typeof sizeClasses;
  show?: boolean;
  className?: string;
  "aria-label"?: string;
}

/**
 * Animated spinner for loading states.
 *
 * Thin wrapper over the shadcn `ui/spinner` that layers on the admin kit's
 * `size` scale, an optional `show` gate, and the primary color.
 */
function Spinner({
  size = "medium",
  show = true,
  className,
  "aria-label": ariaLabel,
}: SpinnerContentProps) {
  if (show === false) {
    return null;
  }
  return (
    <UISpinner
      aria-label={ariaLabel ?? "Loading"}
      className={cn("text-primary", sizeClasses[size], className)}
    />
  );
}

export { Spinner };
