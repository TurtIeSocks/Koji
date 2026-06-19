import * as React from "react";
import type { ChoicesProps, InputProps } from "shadmin-core";
import {
  FieldTitle,
  useChoices,
  useChoicesContext,
  useInput,
  ValidationError,
} from "shadmin-core";
import { cn } from "@/lib/utils";
import { sanitizeInputRestProps } from "@/lib/sanitize-input-rest-props";
import {
  Field,
  FieldError,
  FieldLabel,
  FieldSet,
  FieldLegend,
} from "@/components/ui/field";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Skeleton } from "@/components/ui/skeleton";
import { InputHelperText } from "@/components/admin/common/input-helper-text";

interface RadioButtonGroupInputProps
  extends Partial<InputProps>,
    ChoicesProps,
    Omit<
      React.ComponentProps<typeof RadioGroup>,
      "defaultValue" | "onBlur" | "onChange" | "type"
    > {
  row?: boolean;
  options?: React.ComponentProps<typeof RadioGroup>;
}

/**
 * Single-select input rendered as a list of radio buttons, arranged vertically or horizontally.
 *
 * Use `<RadioButtonGroupInput>` when you have a small set of options (2-5) that users should
 * see all at once, like status, priority, or category fields. For longer lists, prefer
 * `<SelectInput>`. Set `row` to `true` for horizontal layout.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/radio-button-group-input RadioButtonGroupInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, TextInput, RadioButtonGroupInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <RadioButtonGroupInput
 *         source="category"
 *         choices={[
 *           { id: 'tech', name: 'Tech' },
 *           { id: 'lifestyle', name: 'Lifestyle' },
 *           { id: 'people', name: 'People' },
 *         ]}
 *       />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function RadioButtonGroupInput(inProps: RadioButtonGroupInputProps) {
  const {
    choices: choicesProp,
    isFetching: isFetchingProp,
    isLoading: isLoadingProp,
    isPending: isPendingProp,
    resource: resourceProp,
    source: sourceProp,

    format,
    onBlur,
    onChange,
    parse,
    validate,
    disabled,
    readOnly,

    optionText,
    optionValue = "id",
    translateChoice,
    disableValue = "disabled",

    className,
    helperText,
    label,
    row,
    options,
    ...rest
  } = inProps;

  const {
    allChoices,
    isPending,
    error: fetchError,
    resource,
    source,
  } = useChoicesContext({
    choices: choicesProp,
    isFetching: isFetchingProp,
    isLoading: isLoadingProp,
    isPending: isPendingProp,
    resource: resourceProp,
    source: sourceProp,
  });

  if (source === undefined) {
    throw new Error(
      `If you're not wrapping the RadioButtonGroupInput inside a ReferenceArrayInput, you must provide the source prop`,
    );
  }

  if (!isPending && !fetchError && allChoices === undefined) {
    throw new Error(
      `If you're not wrapping the RadioButtonGroupInput inside a ReferenceArrayInput, you must provide the choices prop`,
    );
  }

  const { id, field, fieldState, isRequired } = useInput({
    format,
    onBlur,
    onChange,
    parse,
    resource,
    source,
    validate,
    disabled,
    readOnly,
    ...rest,
  });

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  const { getChoiceText, getChoiceValue, getDisableValue } = useChoices({
    optionText,
    optionValue,
    translateChoice,
    disableValue,
  });

  if (isPending) {
    return <Skeleton className="w-full h-9" />;
  }

  return (
    <FieldSet className={className} data-invalid={invalid || undefined}>
      {label && (
        <FieldLegend variant="label">
          <FieldTitle
            label={label}
            source={source}
            resource={resource}
            isRequired={isRequired}
          />
        </FieldLegend>
      )}
      <RadioGroup
        {...sanitizeInputRestProps(rest)}
        {...options}
        // Radix' RadioGroup only deals in string values; coerce here
        // but pass the original (possibly numeric) id back on change.
        value={field.value != null ? String(field.value) : undefined}
        onValueChange={(nextValue) => {
          const choice = allChoices?.find(
            (c) => String(getChoiceValue(c)) === nextValue,
          );
          field.onChange(choice ? getChoiceValue(choice) : nextValue);
        }}
        className={cn("flex", row ? "flex-row gap-4" : "flex-col gap-2")}
        disabled={disabled || readOnly}
      >
        {allChoices?.map((choice) => {
          const value = getChoiceValue(choice);
          const stringValue = String(value);
          const isDisabled = disabled || readOnly || getDisableValue(choice);

          return (
            <Field
              key={stringValue}
              orientation="horizontal"
              data-disabled={isDisabled || undefined}
            >
              <RadioGroupItem
                value={stringValue}
                id={`${id}-${stringValue}`}
                disabled={isDisabled}
              />
              <FieldLabel
                htmlFor={`${id}-${stringValue}`}
                className="font-normal"
              >
                {getChoiceText(choice)}
              </FieldLabel>
            </Field>
          );
        })}
      </RadioGroup>
      <InputHelperText helperText={helperText ?? fetchError?.message} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </FieldSet>
  );
}

export { RadioButtonGroupInput, type RadioButtonGroupInputProps };
