import * as React from "react";
import { useCallback, useEffect, useMemo, useRef } from "react";
import debounce from "lodash/debounce";
import { Check, ChevronsUpDown } from "lucide-react";
import { areIdsEqual } from "@/lib/are-ids-equal";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import type {
  ChoicesProps,
  InputProps,
  SupportCreateSuggestionOptions,
} from "shadmin-core";
import {
  useChoices,
  useChoicesContext,
  useGetRecordRepresentation,
  useInput,
  useTranslate,
  FieldTitle,
  useEvent,
  useSupportCreateSuggestion,
  ValidationError,
} from "shadmin-core";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { PopoverPrimitive } from "@/components/ui/popover";
import type { UnknownValue } from "@/lib/unknown-types";

/**
 * Form control that lets users choose a value from a list using a dropdown with autocompletion.
 *
 * This input allows editing scalar values with a searchable dropdown interface. It supports creating
 * new choices on the fly and works seamlessly inside ReferenceInput for editing foreign key relationships.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/autocomplete-input AutocompleteInput documentation}
 *
 * @example
 * import {
 *   Create,
 *   SimpleForm,
 *   AutocompleteInput,
 *   ReferenceInput,
 * } from '@/components/admin';
 *
 * const PostCreate = () => (
 *   <Create>
 *     <SimpleForm>
 *       <AutocompleteInput
 *         source="category"
 *         choices={[
 *           { id: 'tech', name: 'Tech' },
 *           { id: 'lifestyle', name: 'Lifestyle' },
 *           { id: 'people', name: 'People' },
 *         ]}
 *       />
 *       <ReferenceInput label="Author" source="author_id" reference="authors">
 *         <AutocompleteInput />
 *       </ReferenceInput>
 *     </SimpleForm>
 *   </Create>
 * );
 */
type AutocompleteInputProps = Omit<InputProps, "source"> &
  Omit<SupportCreateSuggestionOptions, "handleChange" | "filter"> &
  Partial<Pick<InputProps, "source">> &
  ChoicesProps & {
    className?: string;
    debounce?: number;
    disableValue?: string;
    filterToQuery?: (searchText: string) => UnknownValue;
    translateChoice?: boolean;
    placeholder?: string;
    /** Custom display text for the selected value in the trigger button. Must return a string. */
    inputText?: (option: UnknownValue) => string;
    /** When true, only show choices matching the current value in the dropdown. */
    limitChoicesToValue?: boolean;
    /** Cap the number of dropdown items shown. */
    suggestionLimit?: number;
    /** Content to show when no choices match the filter. Defaults to ra.navigation.no_results. */
    noOptionsText?: React.ReactNode;
    /** Underlying value used when clearing the selection. Defaults to "". */
    emptyValue?: string | number;
    /** When true, clear the filter text on input blur. */
    clearOnBlur?: boolean;
    /** When true, open the dropdown on input focus. */
    openOnFocus?: boolean;
    /** When true, select all text in the input on focus. */
    selectOnFocus?: boolean;
    /** When true, Home/End keys scroll the listbox to first/last item. */
    handleHomeEndKeys?: boolean;
    /** Custom equality check between a choice and the current value. */
    isOptionEqualToValue?: (
      option: UnknownValue,
      value: UnknownValue,
    ) => boolean;
    /** Custom filter function; replaces default substring match. */
    matchSuggestion?: (filter: string, choice: UnknownValue) => boolean;
    /** Gate controlling whether the dropdown opens at all. */
    shouldRenderSuggestions?: (filter: string) => boolean;
    /** Label of a "(none)" entry prepended when the field is not required. */
    emptyText?: string;
    /** Called with the current filter text on every keystroke. For server-side filtering outside ReferenceInput. No debounce applied — caller decides. */
    setFilter?: (filter: string) => void;
  } & Pick<PopoverPrimitive.PopoverProps, "modal">;

function AutocompleteInput(props: AutocompleteInputProps) {
  const {
    debounce: debounceDelay = 250,
    filterToQuery = DefaultFilterToQuery,
    inputText,
    create,
    createValue,
    createLabel,
    createHintValue,
    createItemLabel,
    onCreate,
    optionText,
    modal,
    limitChoicesToValue = false,
    suggestionLimit,
    noOptionsText,
    emptyValue = "",
    clearOnBlur = false,
    openOnFocus = false,
    selectOnFocus = false,
    handleHomeEndKeys = false,
    isOptionEqualToValue,
    matchSuggestion,
    shouldRenderSuggestions,
    emptyText: emptyTextProp = "",
    setFilter,
  } = props;
  const {
    allChoices = [],
    source,
    resource,
    isFromReference,
    setFilters,
  } = useChoicesContext(props);
  const { id, field, fieldState, isRequired } = useInput({ ...props, source });
  const translate = useTranslate();
  const { placeholder = translate("ra.action.search", { _: "Search..." }) } =
    props;

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  const getRecordRepresentation = useGetRecordRepresentation(resource);
  const { getChoiceText, getChoiceValue } = useChoices({
    optionText:
      props.optionText ?? (isFromReference ? getRecordRepresentation : "name"),
    optionValue: props.optionValue ?? "id",
    disableValue: props.disableValue,
    translateChoice: props.translateChoice ?? !isFromReference,
  });

  const [filterValue, setFilterValue] = React.useState("");
  const listRef = React.useRef<HTMLDivElement>(null);

  // Keep stable references to dependencies so the debounced setter doesn't
  // need to be recreated on every render (which would defeat debouncing).
  const setFiltersRef = useRef(setFilters);
  const filterToQueryRef = useRef(filterToQuery);
  useEffect(() => {
    setFiltersRef.current = setFilters;
  }, [setFilters]);
  useEffect(() => {
    filterToQueryRef.current = filterToQuery;
  }, [filterToQuery]);

  const debouncedSetFilters = useMemo(
    () =>
      debounce((filter: string) => {
        setFiltersRef.current(filterToQueryRef.current(filter));
      }, debounceDelay),
    [debounceDelay],
  );

  // Cancel any in-flight debounced call when the component unmounts to
  // avoid `setState` on an unmounted component during late dataProvider
  // responses.
  useEffect(() => () => debouncedSetFilters.cancel(), [debouncedSetFilters]);

  // Prop 10: Reset filter when field.value changes externally (e.g. form reset).
  // biome-ignore lint/correctness/useExhaustiveDependencies: field.value is the intentional trigger (reset on external value change); the body only cancels/clears and does not read it.
  useEffect(() => {
    debouncedSetFilters.cancel();
    setFilterValue("");
  }, [field.value, debouncedSetFilters]);

  const [open, setOpen] = React.useState(false);
  const listboxId = React.useId();

  // Equality helper: use isOptionEqualToValue when provided, else areIdsEqual.
  const isEqual = useCallback(
    (choiceVal: UnknownValue, fieldVal: UnknownValue) => {
      if (isOptionEqualToValue) {
        return isOptionEqualToValue(choiceVal, fieldVal);
      }
      return areIdsEqual(choiceVal, fieldVal);
    },
    [isOptionEqualToValue],
  );

  const selectedChoice = allChoices.find((choice) =>
    isEqual(getChoiceValue(choice), field.value),
  );

  const getInputText = useCallback(
    (selectedChoice: UnknownValue) => {
      if (typeof inputText === "function") {
        return inputText(selectedChoice);
      }
      return getChoiceText(selectedChoice);
    },
    [inputText, getChoiceText],
  );

  const handleOpenChange = useEvent((isOpen: boolean) => {
    setOpen(isOpen);
    // Reset the filter when the popover is closed. Run immediately
    // (no debounce) so re-opening the popover always shows the full list.
    if (!isOpen) {
      debouncedSetFilters.cancel();
      setFilters(filterToQuery(""));
    }
  });

  const handleChange = useCallback(
    (choice: UnknownValue) => {
      if (isEqual(getChoiceValue(choice), field.value) && !isRequired) {
        // Prop 4: use emptyValue instead of hardcoded "".
        field.onChange(emptyValue);
        setFilterValue("");
        if (isFromReference) {
          debouncedSetFilters.cancel();
          setFilters(filterToQuery(""));
        }
        setOpen(false);
        return;
      }
      field.onChange(getChoiceValue(choice));
      setOpen(false);
    },
    [
      field,
      getChoiceValue,
      isRequired,
      isEqual,
      emptyValue,
      isFromReference,
      setFilters,
      filterToQuery,
      debouncedSetFilters,
    ],
  );

  const {
    getCreateItem,
    handleChange: handleChangeWithCreateSupport,
    createElement,
    getOptionDisabled,
  } = useSupportCreateSuggestion({
    create,
    createLabel,
    createValue,
    createHintValue,
    createItemLabel,
    onCreate,
    handleChange,
    optionText,
    filter: filterValue,
  });

  const createItem =
    (create || onCreate) && (filterValue !== "" || createLabel)
      ? getCreateItem(filterValue)
      : null;

  // Prop 1: limitChoicesToValue — filter to only the matching choice.
  let availableChoices = limitChoicesToValue
    ? allChoices.filter((c) => isEqual(getChoiceValue(c), field.value))
    : allChoices;

  // matchSuggestion: when provided, apply custom filtering for non-reference inputs.
  // (Reference inputs delegate filtering to the data provider via debouncedSetFilters.)
  if (matchSuggestion && !isFromReference && filterValue !== "") {
    availableChoices = availableChoices.filter((choice) =>
      matchSuggestion(filterValue, choice),
    );
  }

  // Prop 2: suggestionLimit — cap choices after filtering.
  if (suggestionLimit !== undefined) {
    availableChoices = availableChoices.slice(0, suggestionLimit);
  }

  let finalChoices = availableChoices;
  if (createItem) {
    finalChoices = [...finalChoices, createItem];
  }

  // Prop 3: noOptionsText — fall back to translated string.
  const noMatchText =
    noOptionsText ??
    translate("ra.navigation.no_results", { _: "No matching item found." });

  // shouldRenderSuggestions: gate on the effective open state.
  const effectiveOpen =
    open && (!shouldRenderSuggestions || shouldRenderSuggestions(filterValue));

  // Prop 8: handleHomeEndKeys handler for CommandInput.
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (!handleHomeEndKeys) return;
      if (e.key === "Home") {
        e.preventDefault();
        listRef.current?.scrollTo(0, 0);
      } else if (e.key === "End") {
        e.preventDefault();
        listRef.current?.scrollTo(0, listRef.current.scrollHeight);
      }
    },
    [handleHomeEndKeys],
  );

  // Prop 6 + 7: openOnFocus / selectOnFocus handler for CommandInput.
  const handleFocus = useCallback(
    (e: React.FocusEvent<HTMLInputElement>) => {
      if (openOnFocus) setOpen(true);
      if (selectOnFocus) e.currentTarget.select();
    },
    [openOnFocus, selectOnFocus],
  );

  // Prop 5: clearOnBlur handler for CommandInput.
  const handleBlur = useCallback(() => {
    if (clearOnBlur) setFilterValue("");
  }, [clearOnBlur]);

  return (
    <>
      <Field className={props.className} data-invalid={invalid || undefined}>
        {props.label !== false && (
          <FieldLabel htmlFor={id}>
            <FieldTitle
              label={props.label}
              source={props.source ?? source}
              resource={resource}
              isRequired={isRequired}
            />
          </FieldLabel>
        )}
        <Popover
          open={effectiveOpen}
          onOpenChange={handleOpenChange}
          modal={modal}
        >
          <PopoverTrigger asChild>
            <Button
              id={id}
              variant="outline"
              role="combobox"
              aria-expanded={open}
              aria-controls={listboxId}
              aria-invalid={invalid || undefined}
              className="w-full justify-between h-auto py-1.75 font-normal"
            >
              {selectedChoice ? (
                getInputText(selectedChoice)
              ) : (
                <span className="text-muted-foreground">{placeholder}</span>
              )}
              <ChevronsUpDown className="shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-full max-w-(--radix-popover-trigger-width) p-0">
            {/* We handle the filtering ourselves when matchSuggestion is provided */}
            <Command shouldFilter={!isFromReference && !matchSuggestion}>
              <CommandInput
                placeholder={translate("ra.action.search", {
                  _: "Search...",
                })}
                value={filterValue}
                onValueChange={(filter) => {
                  setFilterValue(filter);
                  setFilter?.(filter);
                  requestAnimationFrame(() => {
                    listRef.current?.scrollTo(0, 0);
                  });
                  // We don't want the ChoicesContext to filter the choices if the input
                  // is not from a reference as it would also filter out the selected values
                  if (isFromReference) {
                    debouncedSetFilters(filter);
                  }
                }}
                onKeyDown={handleKeyDown}
                onFocus={handleFocus}
                onBlur={handleBlur}
              />
              <CommandList id={listboxId} ref={listRef}>
                {/* Prop 3: noOptionsText */}
                <CommandEmpty>{noMatchText}</CommandEmpty>
                <CommandGroup>
                  {/* emptyText: "(none)" entry when prop is set and field is not required */}
                  {emptyTextProp !== "" && !isRequired && (
                    <CommandItem
                      value={`__empty__${emptyTextProp}`}
                      onSelect={() => {
                        field.onChange(emptyValue);
                        setFilterValue("");
                        setOpen(false);
                      }}
                      className="text-muted-foreground"
                    >
                      <Check
                        className={cn(
                          "mr-2 size-4",
                          field.value === emptyValue ||
                            field.value == null ||
                            field.value === ""
                            ? "opacity-100"
                            : "opacity-0",
                        )}
                      />
                      {emptyTextProp}
                    </CommandItem>
                  )}
                  {finalChoices.map((choice) => {
                    const isCreateItem =
                      !!createItem && choice?.id === createItem.id;
                    const disabled = getOptionDisabled(choice);

                    const choiceText = getChoiceText(
                      isCreateItem ? createItem : choice,
                    );

                    return (
                      <CommandItem
                        key={getChoiceValue(choice)}
                        keywords={
                          isCreateItem || React.isValidElement(choiceText)
                            ? undefined
                            : [choiceText]
                        }
                        value={
                          isCreateItem
                            ? // if it's the create option, include the filter value so it is shown in the command input
                              // characters before and after the filter value are required
                              // to show the option when the filter value starts or ends with a space
                              `?${filterValue}?`
                            : getChoiceValue(choice)
                        }
                        onSelect={() => handleChangeWithCreateSupport(choice)}
                        disabled={disabled}
                      >
                        <Check
                          className={cn(
                            "mr-2 size-4",
                            isEqual(field.value, getChoiceValue(choice))
                              ? "opacity-100"
                              : "opacity-0",
                          )}
                        />
                        {choiceText}
                      </CommandItem>
                    );
                  })}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
        <InputHelperText helperText={props.helperText} />
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

const DefaultFilterToQuery = (searchText: string) => ({ q: searchText });

export { AutocompleteInput, type AutocompleteInputProps };
