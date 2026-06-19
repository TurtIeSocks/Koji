import * as React from "react";
import type { ComponentProps } from "react";
import type { ChoicesProps, InputProps } from "shadmin-core";
import {
  FieldTitle,
  useChoices,
  useChoicesContext,
  useGetRecordRepresentation,
  useInput,
  ValidationError,
} from "shadmin-core";
import { cn } from "@/lib/utils";
import {
  Field,
  FieldError,
  FieldLabel,
  FieldSet,
  FieldLegend,
} from "@/components/ui/field";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { InputHelperText } from "@/components/admin/common/input-helper-text";

/**
 * Multi-select input rendered as a list of checkboxes, arranged vertically or horizontally.
 *
 * Use `<CheckboxGroupInput>` when you have a small set of options (2-6) and want to allow users
 * to pick several at once. The form value is an array of choice ids. For longer lists, prefer
 * `<SelectArrayInput>` or `<AutocompleteArrayInput>`. Set `row` to `true` for horizontal layout.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/checkbox-group-input CheckboxGroupInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, TextInput, CheckboxGroupInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <CheckboxGroupInput
 *         source="tags"
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
function CheckboxGroupInput(inProps: CheckboxGroupInputProps) {
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
    row = false,
    options,
    labelPlacement = "end",
    ...rest
  } = inProps;

  const {
    allChoices,
    isPending,
    error: fetchError,
    resource,
    source,
    isFromReference,
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
      `If you're not wrapping the CheckboxGroupInput inside a ReferenceArrayInput, you must provide the source prop`,
    );
  }

  if (!isPending && !fetchError && allChoices === undefined) {
    throw new Error(
      `If you're not wrapping the CheckboxGroupInput inside a ReferenceArrayInput, you must provide the choices prop`,
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

  const getRecordRepresentation = useGetRecordRepresentation(resource);
  const { getChoiceText, getChoiceValue, getDisableValue } = useChoices({
    optionText:
      optionText ?? (isFromReference ? getRecordRepresentation : undefined),
    optionValue,
    translateChoice: translateChoice ?? !isFromReference,
    disableValue,
  });

  if (isPending) {
    return <Skeleton className="w-full h-9" />;
  }

  const value: Array<string | number> = Array.isArray(field.value)
    ? field.value
    : [];

  const handleCheck = (choiceValue: string | number, checked: boolean) => {
    const next = checked
      ? [...value, choiceValue]
      : value.filter((v) => v !== choiceValue);
    field.onChange(next);
    field.onBlur();
  };

  return (
    <FieldSet className={className} data-invalid={invalid || undefined}>
      {label !== false && (
        <FieldLegend variant="label">
          <FieldTitle
            label={label}
            source={source}
            resource={resource}
            isRequired={isRequired}
          />
        </FieldLegend>
      )}
      <div className={cn("flex", row ? "flex-row gap-4" : "flex-col gap-2")}>
        {allChoices?.map((choice) => {
          const choiceValue = getChoiceValue(choice);
          const isChoiceDisabled =
            disabled || readOnly || getDisableValue(choice);
          const choiceId = `${id}-${choiceValue}`;

          const checkbox = (
            <Checkbox
              id={choiceId}
              checked={value.includes(choiceValue)}
              disabled={isChoiceDisabled}
              onCheckedChange={(checked) =>
                handleCheck(choiceValue, checked === true)
              }
              {...options}
            />
          );
          const labelEl = (
            <FieldLabel htmlFor={choiceId} className="font-normal">
              {getChoiceText(choice)}
            </FieldLabel>
          );

          const isTop = labelPlacement === "top";
          const isBottom = labelPlacement === "bottom";
          const isStart = labelPlacement === "start";

          return (
            <Field
              key={choiceValue}
              orientation={isTop || isBottom ? "vertical" : "horizontal"}
              data-disabled={isChoiceDisabled || undefined}
              className={cn(
                isTop || isBottom ? "items-start gap-y-1" : undefined,
                isTop && "flex-col-reverse",
              )}
            >
              {isStart || isTop ? labelEl : null}
              {checkbox}
              {!isStart && !isTop ? labelEl : null}
            </Field>
          );
        })}
      </div>
      <InputHelperText helperText={helperText ?? fetchError?.message} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </FieldSet>
  );
}

interface CheckboxGroupInputProps
  extends Partial<InputProps>,
    ChoicesProps,
    Omit<
      React.HTMLAttributes<HTMLDivElement>,
      "defaultValue" | "onBlur" | "onChange"
    > {
  row?: boolean;
  options?: ComponentProps<typeof Checkbox>;
  labelPlacement?: "end" | "start" | "top" | "bottom";
}

export { CheckboxGroupInput, type CheckboxGroupInputProps };
