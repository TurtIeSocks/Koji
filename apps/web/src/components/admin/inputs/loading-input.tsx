import type { ReactNode } from "react";
import { useTimeout } from "shadmin-core";

import { cn } from "@/lib/utils";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Spinner } from "@/components/admin/feedback/spinner";

interface LoadingInputProps {
  fullWidth?: boolean;
  label?: ReactNode;
  helperText?: ReactNode;
  size?: "default" | "sm";
  timeout?: number;
  className?: string;
}

/**
 * An input placeholder with a loading indicator.
 *
 * Renders the same outer shape as a `<TextInput>` so it can be swapped in for one
 * while the real data loads, avoiding visual jumps. The input is disabled and a
 * spinner appears inside it once the `timeout` (default `1000`ms) has elapsed.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/loading-input LoadingInput documentation}
 *
 * @example
 * import { LoadingInput } from '@/components/admin';
 *
 * const MyInput = () => {
 *   const { isLoading } = useGetSomething();
 *   if (isLoading) return <LoadingInput label="Title" />;
 *   return <TextInput source="title" />;
 * };
 */
function LoadingInput({
  fullWidth,
  label,
  helperText,
  size = "default",
  timeout = 1000,
  className,
}: LoadingInputProps) {
  const ready = useTimeout(timeout);

  return (
    <fieldset
      className={cn("grid gap-2", fullWidth ? "w-full" : undefined, className)}
      data-slot="loading-input"
    >
      {label != null ? <Label aria-disabled>{label}</Label> : null}
      <div className="relative">
        <Input
          disabled
          aria-busy
          readOnly
          value=""
          className={cn("pr-10", size === "sm" ? "h-8 text-sm" : undefined)}
        />
        <span className="absolute inset-y-0 right-2 flex items-center text-muted-foreground">
          {ready ? (
            <Spinner size="small" className="size-5" />
          ) : (
            <span style={{ width: 20, height: 20 }} />
          )}
        </span>
      </div>
      {helperText ? (
        <p className="text-muted-foreground text-sm">{helperText}</p>
      ) : null}
    </fieldset>
  );
}

export { LoadingInput, type LoadingInputProps };
