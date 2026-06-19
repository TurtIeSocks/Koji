import * as React from "react";
import type { InputProps } from "shadmin-core";
import {
  FieldTitle,
  isRequired,
  useSourceContext,
  sanitizeInputRestProps,
  ArrayInputBase,
  ValidationError,
} from "shadmin-core";
import { useFormContext, useFormState } from "react-hook-form";

import { cn } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";

/**
 * Creates a list of sub-forms for editing arrays of data embedded inside a record.
 *
 * Use `<ArrayInput>` when you need to edit array fields like order items, tags, or any
 * repeatable embedded data. Requires a form iterator child (typically `<SimpleFormIterator>`)
 * to render and manage individual array items.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/array-input ArrayInput documentation}
 *
 * @example
 * import {
 *   Edit,
 *   SimpleForm,
 *   TextInput,
 *   NumberInput,
 *   ArrayInput,
 *   SimpleFormIterator,
 * } from '@/components/admin';
 *
 * const OrderEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="customer" />
 *       <TextInput source="date" type="date" />
 *       <ArrayInput source="items">
 *         <SimpleFormIterator inline>
 *           <TextInput source="name" />
 *           <NumberInput source="price" />
 *           <NumberInput source="quantity" />
 *         </SimpleFormIterator>
 *       </ArrayInput>
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function ArrayInput(props: ArrayInputProps) {
  const {
    className,
    defaultValue = [],
    label,
    isPending,
    loading,
    children,
    helperText,
    resource: resourceFromProps,
    source: arraySource,
    validate,
    ...rest
  } = props;

  const parentSourceContext = useSourceContext();
  const finalSource = parentSourceContext.getSource(arraySource);

  const { getFieldState } = useFormContext();
  const formState = useFormState({ name: finalSource });
  const fieldState = getFieldState(finalSource, formState);
  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  if (isPending) {
    return loading != null ? loading : <Skeleton className="w-full h-9" />;
  }

  return (
    <Field
      className={cn(
        "ra-input",
        `ra-input-${finalSource}`,
        className,
        "w-full flex flex-col gap-2",
      )}
      data-invalid={invalid || undefined}
      {...sanitizeInputRestProps(rest)}
    >
      <FieldLabel className="text-muted-foreground text-sm">
        <FieldTitle
          label={label}
          source={arraySource}
          resource={resourceFromProps}
          isRequired={isRequired(validate)}
        />
      </FieldLabel>
      <ArrayInputBase {...props} defaultValue={defaultValue}>
        {children}
      </ArrayInputBase>

      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </Field>
  );
}

interface ArrayInputProps extends Omit<InputProps, "disabled" | "readOnly"> {
  className?: string;
  children: React.ReactNode;
  isFetching?: boolean;
  isLoading?: boolean;
  isPending?: boolean;
  loading?: React.ReactNode;
}

export { ArrayInput, type ArrayInputProps };
