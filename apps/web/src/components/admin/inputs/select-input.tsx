import { X } from "lucide-react";
import type {
  ChoicesProps,
  InputProps,
  SupportCreateSuggestionOptions,
} from "shadmin-core";
import {
  FieldTitle,
  useChoices,
  useChoicesContext,
  useGetRecordRepresentation,
  useInput,
  useSupportCreateSuggestion,
  useTranslate,
  ValidationError,
} from "shadmin-core";
import type { ComponentProps, ReactElement } from "react";
import { useCallback, useEffect } from "react";

import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import type { UnknownValue } from "@/lib/unknown-types";

/**
 * Dropdown select input for choosing a single value from a list of options.
 *
 * Use `<SelectInput>` for fields with many possible values (5+) like categories, statuses, or
 * countries. Supports creating new options on the fly with the `create` or `onCreate` props.
 * Wrap in `<ReferenceInput>` to select from related resources.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/select-input SelectInput documentation}
 * @see {@link https://ui.shadcn.com/docs/components/select Select documentation}
 *
 * @example
 * import { Edit, SimpleForm, TextInput, SelectInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <SelectInput
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
function SelectInput(props: SelectInputProps) {
  const {
    choices: choicesProp,
    isLoading: isLoadingProp,
    isFetching: isFetchingProp,
    isPending: isPendingProp,
    resource: resourceProp,
    source: sourceProp,

    optionText,
    optionValue,
    disableValue = "disabled",
    translateChoice,
    createValue,
    createHintValue,

    alwaysOn,
    defaultValue,
    format,
    label,
    helperText,
    name,
    onBlur,
    onChange,
    parse,
    validate,
    readOnly,
    disabled,

    className,
    emptyText = "",
    emptyValue = "",
    filter: _filter,
    create,
    createLabel,
    onCreate,
    resettable = true,

    ...rest
  } = props;
  const translate = useTranslate();

  useEffect(() => {
    if (emptyValue == null) {
      throw new Error(
        `emptyValue being set to null or undefined is not supported. Use parse to turn the empty string into null.`,
      );
    }
  }, [emptyValue]);

  const {
    allChoices,
    isPending,
    error: fetchError,
    source,
    resource,
    isFromReference,
  } = useChoicesContext({
    choices: choicesProp,
    isLoading: isLoadingProp,
    isFetching: isFetchingProp,
    isPending: isPendingProp,
    resource: resourceProp,
    source: sourceProp,
  });

  if (source === undefined) {
    throw new Error(
      `If you're not wrapping the SelectInput inside a ReferenceInput, you must provide the source prop`,
    );
  }

  if (!isPending && !fetchError && allChoices === undefined) {
    throw new Error(
      `If you're not wrapping the SelectInput inside a ReferenceInput, you must provide the choices prop`,
    );
  }

  const getRecordRepresentation = useGetRecordRepresentation(resource);
  const { getChoiceText, getChoiceValue, getDisableValue } = useChoices({
    optionText:
      optionText ?? (isFromReference ? getRecordRepresentation : undefined),
    optionValue,
    disableValue,
    translateChoice: translateChoice ?? !isFromReference,
    createValue,
    createHintValue,
  });
  const { id, field, fieldState, isRequired } = useInput({
    alwaysOn,
    defaultValue,
    format,
    label,
    helperText,
    name,
    onBlur,
    onChange,
    parse,
    resource,
    source,
    validate,
    readOnly,
    disabled,
  });

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  const renderEmptyItemOption = useCallback(() => {
    return typeof emptyText === "string"
      ? emptyText === ""
        ? " " // em space, forces the display of an empty line of normal height
        : translate(emptyText, { _: emptyText })
      : emptyText;
  }, [emptyText, translate]);

  const renderMenuItemOption = useCallback(
    (choice: UnknownValue) => getChoiceText(choice),
    [getChoiceText],
  );

  const handleChange = useCallback(
    async (value: string) => {
      if (value === emptyValue) {
        field.onChange(emptyValue);
      } else {
        // The Radix Select API only deals in strings, but the original
        // choice id may be numeric. Stringify both sides for comparison
        // and submit the *original* id type so we don't silently corrupt
        // numeric ids into strings on the wire.
        const choice = allChoices?.find(
          (choice) => String(getChoiceValue(choice)) === value,
        );
        field.onChange(choice ? getChoiceValue(choice) : value);
      }
    },
    [field, getChoiceValue, emptyValue, allChoices],
  );

  const {
    getCreateItem,
    handleChange: handleChangeWithCreateSupport,
    createElement,
  } = useSupportCreateSuggestion({
    create,
    createLabel,
    createValue,
    createHintValue,
    onCreate,
    handleChange,
    optionText,
  });

  if (isPending) {
    return (
      <Field
        className={cn("w-full min-w-20", className)}
        data-invalid={invalid || undefined}
      >
        {label !== "" && label !== false && (
          <FieldLabel htmlFor={id}>
            <FieldTitle
              label={label}
              source={source}
              resource={resourceProp}
              isRequired={isRequired}
            />
          </FieldLabel>
        )}
        <div className="relative">
          <Skeleton className="w-full h-9" />
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

  const createItem = create || onCreate ? getCreateItem() : null;
  let finalChoices = fetchError ? [] : allChoices;
  if (create || onCreate) {
    finalChoices = [...finalChoices, createItem];
  }

  // Handle reset functionality
  const handleReset = (e: React.SyntheticEvent) => {
    e.stopPropagation();
    e.preventDefault();
    field.onChange(emptyValue);
  };

  return (
    <>
      <Field
        className={cn("w-full min-w-20", className)}
        data-invalid={invalid || undefined}
        {...rest}
      >
        {label !== "" && label !== false && (
          <FieldLabel htmlFor={id}>
            <FieldTitle
              label={label}
              source={source}
              resource={resourceProp}
              isRequired={isRequired}
            />
          </FieldLabel>
        )}
        <div className="relative">
          <Select
            //FIXME https://github.com/radix-ui/primitives/issues/3135
            // Setting a key based on the value fixes an issue where onValueChange
            // was called with an empty string when the controlled value was changed.
            // See: https://github.com/radix-ui/primitives/issues/3135#issuecomment-2916908248
            key={`select:${field.value?.toString() ?? emptyValue}`}
            value={field.value?.toString() || emptyValue}
            onValueChange={handleChangeWithCreateSupport}
          >
            <SelectTrigger
              id={id}
              aria-invalid={invalid || undefined}
              className={cn("w-full transition-all hover:bg-accent")}
              disabled={field.disabled}
            >
              <SelectValue placeholder={renderEmptyItemOption()} />

              {resettable && field.value && field.value !== emptyValue ? (
                // <span> instead of <button>: SelectTrigger is itself a <button>
                // and HTML forbids nested interactive controls.
                // biome-ignore lint/a11y/useSemanticElements: nested <button> inside the trigger <button> is invalid HTML; focusable role="button" span with keyboard handler is the accessible alternative
                <span
                  role="button"
                  tabIndex={0}
                  aria-label={translate("ra.action.clear_input_value", {
                    _: "Clear",
                  })}
                  className="p-0 ml-auto pointer-events-auto hover:bg-transparent text-muted-foreground opacity-50 hover:opacity-100"
                  onClick={handleReset}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      handleReset(e);
                    }
                  }}
                >
                  <X className="size-4" />
                </span>
              ) : null}
            </SelectTrigger>
            <SelectContent position="popper">
              {finalChoices?.map((choice) => {
                if (!choice) return null;
                const value = getChoiceValue(choice);
                const isDisabled = getDisableValue(choice);

                return (
                  <SelectItem
                    key={value}
                    value={value?.toString()}
                    disabled={isDisabled}
                  >
                    {renderMenuItemOption(
                      createItem && choice?.id === createItem.id
                        ? createItem
                        : choice,
                    )}
                  </SelectItem>
                );
              })}
            </SelectContent>
          </Select>
        </div>
        <InputHelperText
          helperText={helperText ?? (fetchError as Error | undefined)?.message}
        />
        <FieldError>
          {invalid && errorMessage ? (
            <ValidationError error={errorMessage} />
          ) : null}
        </FieldError>
      </Field>
      {createElement}
    </>
  );
}

type SelectInputProps = ChoicesProps &
  // Source is optional as SelectInput can be used inside a ReferenceInput that already defines the source
  Partial<InputProps> &
  Omit<SupportCreateSuggestionOptions, "handleChange"> & {
    emptyText?: string | ReactElement;
    emptyValue?: string | number | boolean | null;
    onChange?: (value: string) => void;
    resettable?: boolean;
  } & Omit<ComponentProps<typeof Field>, "children">;

export { SelectInput, type SelectInputProps };
