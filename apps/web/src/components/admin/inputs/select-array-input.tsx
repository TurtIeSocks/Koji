import * as React from "react";
import { ChevronDown, X } from "lucide-react";
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
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

/**
 * Multi-select dropdown input for choosing several values from a static list.
 *
 * Use `<SelectArrayInput>` for array fields with many possible values (5+) where you want a compact
 * UI: a button shows selected items as badges and opens a popover with checkboxes. Unlike
 * `<AutocompleteArrayInput>`, this input is not searchable. Wrap in `<ReferenceArrayInput>` to
 * select from related resources.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/select-array-input SelectArrayInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, SelectArrayInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <SelectArrayInput
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
function SelectArrayInput(props: SelectArrayInputProps) {
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
    create,
    createLabel,
    createItemLabel,
    onCreate,
    InputLabelProps,

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
    placeholder,
    className,

    ...rest
  } = props;
  const translate = useTranslate();
  const [open, setOpen] = React.useState(false);
  const listboxId = React.useId();

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
      `If you're not wrapping the SelectArrayInput inside a ReferenceArrayInput, you must provide the source prop`,
    );
  }

  if (!isPending && !fetchError && allChoices === undefined) {
    throw new Error(
      `If you're not wrapping the SelectArrayInput inside a ReferenceArrayInput, you must provide the choices prop`,
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

  const value: Array<string | number> = Array.isArray(field.value)
    ? field.value
    : field.value != null
      ? [field.value]
      : [];

  const handleToggle = (choiceValue: string | number) => {
    const isSelected = value.some((v) => String(v) === String(choiceValue));
    const next = isSelected
      ? value.filter((v) => String(v) !== String(choiceValue))
      : [...value, choiceValue];
    field.onChange(next);
  };

  const {
    getCreateItem,
    handleChange: handleChangeWithCreateSupport,
    createElement,
  } = useSupportCreateSuggestion({
    create,
    createLabel,
    createItemLabel,
    createValue,
    createHintValue,
    onCreate,
    handleChange: async (eventOrValue) => {
      // SelectArrayInput appends the freshly-created item to the array
      const newValue =
        typeof eventOrValue === "object" &&
        eventOrValue !== null &&
        "target" in eventOrValue
          ? (eventOrValue as React.ChangeEvent<HTMLInputElement>).target.value
          : (eventOrValue as string | number);
      field.onChange([...value, newValue]);
    },
    optionText,
  });

  if (isPending) {
    return (
      <Field
        className={cn("w-full min-w-20", className)}
        data-invalid={invalid || undefined}
      >
        {label !== false && label !== "" && (
          <FieldLabel htmlFor={id}>
            <FieldTitle
              label={label}
              source={source}
              resource={resourceProp}
              isRequired={isRequired}
            />
          </FieldLabel>
        )}
        <Skeleton className="w-full h-9" />
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

  const handleClear = (e: React.SyntheticEvent) => {
    e.stopPropagation();
    field.onChange([]);
  };

  let choicesArr = fetchError ? [] : (allChoices ?? []);
  if (createItem) {
    choicesArr = [...choicesArr, createItem];
  }
  const selectedChoices = choicesArr.filter((choice) =>
    value.some((v) => String(v) === String(getChoiceValue(choice))),
  );

  return (
    <Field
      className={cn("w-full min-w-20", className)}
      data-invalid={invalid || undefined}
      {...rest}
    >
      {label !== false && label !== "" && (
        <FieldLabel htmlFor={id} {...InputLabelProps}>
          <FieldTitle
            label={label}
            source={source}
            resource={resourceProp}
            isRequired={isRequired}
          />
        </FieldLabel>
      )}
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            id={id}
            type="button"
            variant="outline"
            role="combobox"
            aria-expanded={open}
            aria-controls={listboxId}
            aria-invalid={invalid || undefined}
            disabled={disabled || readOnly}
            className={cn(
              "w-full justify-between font-normal hover:bg-accent min-h-9 h-auto py-1.5",
            )}
          >
            <div className="flex flex-wrap gap-1 items-center flex-1 min-w-0">
              {selectedChoices.length === 0 ? (
                <span className="text-muted-foreground">
                  {placeholder ?? " "}
                </span>
              ) : (
                selectedChoices.map((choice) => (
                  <Badge
                    key={String(getChoiceValue(choice))}
                    variant="outline"
                    className="font-normal"
                  >
                    {getChoiceText(choice)}
                  </Badge>
                ))
              )}
            </div>
            <div className="flex items-center gap-1 ml-2 shrink-0">
              {selectedChoices.length > 0 && !disabled && !readOnly ? (
                // biome-ignore lint/a11y/useSemanticElements: the combobox trigger is itself a <button>; a nested <button> is invalid HTML, so the clear control is a focusable role="button" span
                <span
                  role="button"
                  tabIndex={0}
                  className="p-0 pointer-events-auto hover:bg-transparent text-muted-foreground opacity-50 hover:opacity-100"
                  onClick={handleClear}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      handleClear(e);
                    }
                  }}
                  onPointerDown={(e) => e.stopPropagation()}
                  aria-label={translate("ra.action.clear_input_value", {
                    _: "Clear value",
                  })}
                >
                  <X className="size-4" />
                </span>
              ) : null}
              <ChevronDown className="size-4 opacity-50" aria-hidden="true" />
            </div>
          </Button>
        </PopoverTrigger>
        <PopoverContent
          className="p-0 w-(--radix-popover-trigger-width)"
          align="start"
        >
          <div
            id={listboxId}
            role="listbox"
            aria-multiselectable="true"
            className="flex flex-col max-h-64 overflow-auto py-1"
          >
            {choicesArr.length === 0 ? (
              <p className="px-3 py-2 text-sm text-muted-foreground">
                {translate("ra.navigation.no_results", { _: "No results" })}
              </p>
            ) : (
              choicesArr.map((choice) => {
                const choiceValue = getChoiceValue(choice);
                const choiceText = getChoiceText(choice);
                const isChoiceDisabled = getDisableValue(choice);
                const isSelected = value.some(
                  (v) => String(v) === String(choiceValue),
                );
                const itemId = `${id}-${choiceValue}`;
                const isCreate = createItem != null && choice === createItem;
                if (isCreate) {
                  return (
                    <button
                      key={String(choiceValue)}
                      type="button"
                      className="flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer hover:bg-accent text-left"
                      onClick={() => handleChangeWithCreateSupport(choiceValue)}
                    >
                      <span className="flex-1">{choiceText}</span>
                    </button>
                  );
                }
                return (
                  <label
                    key={String(choiceValue)}
                    htmlFor={itemId}
                    className={cn(
                      "flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer hover:bg-accent",
                      isChoiceDisabled && "opacity-50 cursor-not-allowed",
                    )}
                  >
                    <Checkbox
                      id={itemId}
                      checked={isSelected}
                      disabled={isChoiceDisabled}
                      onCheckedChange={() => handleToggle(choiceValue)}
                    />
                    <span className="flex-1">{choiceText}</span>
                  </label>
                );
              })
            )}
          </div>
        </PopoverContent>
      </Popover>
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
      {createElement}
    </Field>
  );
}

type SelectArrayInputProps = ChoicesProps &
  // Source is optional as SelectArrayInput can be used inside a ReferenceArrayInput that already defines the source
  Partial<InputProps> &
  Omit<Partial<SupportCreateSuggestionOptions>, "handleChange" | "filter"> & {
    className?: string;
    placeholder?: string;
    InputLabelProps?: React.ComponentProps<typeof FieldLabel>;
  };

export { SelectArrayInput, type SelectArrayInputProps };
