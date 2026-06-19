import * as React from "react";
import type { InputProps } from "shadmin-core";
import {
  FieldTitle,
  useInput,
  useResourceContext,
  ValidationError,
} from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { cn } from "@/lib/utils";

interface ColorInputProps
  extends InputProps,
    Omit<
      React.ComponentProps<"input">,
      "defaultValue" | "onBlur" | "onChange" | "type"
    > {
  swatches?: readonly string[];
}

/**
 * Input component for CSS color values using a native `<input type="color">`.
 *
 * Designed for the property resource's `color` category. A full oklch popover
 * picker can be vendored in a follow-on task once the `ui/color-picker`
 * primitive is available.
 *
 * @example
 * import { ColorInput } from '@/components/admin';
 *
 * <ColorInput source="color" swatches={["#ef4444", "#3b82f6", "#22c55e"]} />
 */
function ColorInput(props: ColorInputProps) {
  const {
    label,
    source,
    className,
    resource: resourceProp,
    helperText,
    swatches,
    disabled,
  } = props;
  const resource = useResourceContext({ resource: resourceProp });

  // Strip react-admin handler props before forwarding to useInput so it
  // doesn't receive the onChange/onBlur from the component's own prop spread.
  const { onChange: _sc, onBlur: _sb, ...sansHandlers } = props;
  void _sc;
  void _sb;

  const { id, field, fieldState, isRequired } = useInput(sansHandlers);
  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;
  const value = (field.value as string | undefined) ?? "#000000";

  return (
    <Field className={className} data-invalid={invalid || undefined}>
      {label !== false && (
        <FieldLabel htmlFor={id}>
          <FieldTitle
            label={label}
            source={source}
            resource={resource}
            isRequired={isRequired}
          />
        </FieldLabel>
      )}
      <div className="flex items-center gap-2">
        <input
          id={id}
          type="color"
          value={value}
          disabled={disabled}
          aria-invalid={invalid || undefined}
          onChange={(e) => field.onChange(e.target.value)}
          onBlur={field.onBlur}
          className={cn(
            "h-9 w-16 cursor-pointer rounded border border-input bg-transparent p-1",
            disabled && "cursor-not-allowed opacity-50",
          )}
        />
        <span className="text-sm text-muted-foreground">{value}</span>
        {swatches?.map((s) => (
          <button
            key={s}
            type="button"
            aria-label={`Select color ${s}`}
            onClick={() => field.onChange(s)}
            disabled={disabled}
            className="h-6 w-6 rounded border border-border"
            style={{ backgroundColor: s }}
          />
        ))}
      </div>
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </Field>
  );
}

export { ColorInput, type ColorInputProps };
