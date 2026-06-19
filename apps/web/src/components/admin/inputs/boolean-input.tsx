import React, { useCallback } from "react";
import { Switch } from "@/components/ui/switch";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import type { Validator } from "shadmin-core";
import { useInput, FieldTitle, ValidationError } from "shadmin-core";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import type { UnknownValue } from "@/lib/unknown-types";
import type { ComponentProps } from "react";

/**
 * Toggle switch for boolean (true/false) values.
 *
 * Use `<BooleanInput>` for binary settings like "is published", "is active", or feature flags.
 * Leverages shadcn's Switch component for a native-looking toggle. Note: this input doesn't
 * support `null` values—use `<SelectInput>` for nullable booleans.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/boolean-input BooleanInput documentation}
 * @see {@link https://ui.shadcn.com/docs/components/switch Switch documentation}
 *
 * @example
 * import {
 *   Edit,
 *   SimpleForm,
 *   BooleanInput,
 *   TextInput,
 * } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <BooleanInput source="is_published" />
 *       <BooleanInput source="allow_comments" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function BooleanInput(props: BooleanInputProps) {
  const {
    className,
    defaultValue = false,
    format,
    label,
    helperText,
    onBlur,
    onChange,
    onFocus,
    readOnly,
    disabled,
    parse,
    resource,
    source,
    validate,
    options,
    ...rest
  } = props;
  const { id, field, fieldState, isRequired } = useInput({
    defaultValue,
    format,
    parse,
    resource,
    source,
    onBlur,
    onChange,
    type: "checkbox",
    validate,
    disabled,
    readOnly,
    ...rest,
  });

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  const handleChange = useCallback(
    (checked: boolean) => {
      field.onChange(checked);
      // Ensure field is considered as touched
      field.onBlur();
    },
    [field],
  );

  return (
    <Field className={className} data-invalid={invalid || undefined}>
      <div className="flex items-center gap-2">
        <Switch
          id={id}
          aria-invalid={invalid || undefined}
          checked={Boolean(field.value)}
          onFocus={onFocus}
          onCheckedChange={handleChange}
          disabled={disabled || readOnly}
          {...options}
        />
        <FieldLabel htmlFor={id}>
          <FieldTitle
            label={label}
            source={source}
            resource={resource}
            isRequired={isRequired}
          />
        </FieldLabel>
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

interface BooleanInputProps {
  className?: string;
  defaultValue?: boolean;
  format?: (value: UnknownValue) => boolean;
  helperText?: React.ReactNode;
  label?: React.ReactNode;
  onBlur?: (event: React.FocusEvent<HTMLButtonElement>) => void;
  onChange?: (value: boolean) => void;
  onFocus?: (event: React.FocusEvent<HTMLButtonElement>) => void;
  readOnly?: boolean;
  disabled?: boolean;
  parse?: (value: UnknownValue) => UnknownValue;
  resource?: string;
  source: string;
  validate?: Validator | Validator[];
  options?: ComponentProps<typeof Switch>;
}

export { BooleanInput, type BooleanInputProps };
