import * as React from "react";
import { useCallback, useEffect, useMemo, useRef } from "react";
import debounce from "lodash/debounce";
import { X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Command,
  CommandGroup,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
  Popover,
  PopoverAnchor,
  PopoverContent,
  PopoverPrimitive,
} from "@/components/ui/popover";
import { Command as CommandPrimitive } from "cmdk";
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
import { areIdsEqual } from "@/lib/are-ids-equal";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import type { UnknownValue } from "@/lib/unknown-types";

/**
 * Form control that lets users choose multiple values from a list using a dropdown with autocompletion.
 *
 * This input allows editing array values with a searchable dropdown interface and displays selected items as removable badges.
 * Works seamlessly inside ReferenceArrayInput for editing many-to-many relationships.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/autocomplete-array-input AutocompleteArrayInput documentation}
 *
 * @example
 * import {
 *   Create,
 *   SimpleForm,
 *   AutocompleteArrayInput,
 *   ReferenceArrayInput,
 * } from '@/components/admin';
 *
 * const PostCreate = () => (
 *   <Create>
 *     <SimpleForm>
 *       <AutocompleteArrayInput
 *         source="tags"
 *         choices={[
 *           { id: 'tech', name: 'Tech' },
 *           { id: 'news', name: 'News' },
 *           { id: 'lifestyle', name: 'Lifestyle' },
 *         ]}
 *       />
 *       <ReferenceArrayInput source="tag_ids" reference="tags">
 *         <AutocompleteArrayInput />
 *       </ReferenceArrayInput>
 *     </SimpleForm>
 *   </Create>
 * );
 */
type AutocompleteArrayInputProps = Omit<InputProps, "source"> &
  Omit<SupportCreateSuggestionOptions, "handleChange" | "filter"> &
  Partial<Pick<InputProps, "source">> &
  ChoicesProps & {
    className?: string;
    debounce?: number;
    disableValue?: string;
    filterToQuery?: (searchText: string) => UnknownValue;
    translateChoice?: boolean;
    placeholder?: string;
    /** Custom display text for badge labels. Must return a string. */
    inputText?: (option: UnknownValue) => string;
    /** When true, only show choices that are already selected in the dropdown. */
    limitChoicesToValue?: boolean;
    /** Cap the number of dropdown items shown. */
    suggestionLimit?: number;
    /** Content to show when no choices match the filter. Defaults to ra.navigation.no_results. */
    noOptionsText?: React.ReactNode;
    /** When true, clear the filter text on input blur. */
    clearOnBlur?: boolean;
    /** When true, open the dropdown on input focus. */
    openOnFocus?: boolean;
    /** When true, select all text in the input on focus. */
    selectOnFocus?: boolean;
    /** When true, Home/End keys scroll the listbox to first/last item. */
    handleHomeEndKeys?: boolean;
    /** Custom equality check between a choice and a selected value. */
    isOptionEqualToValue?: (
      option: UnknownValue,
      value: UnknownValue,
    ) => boolean;
    /** Custom filter function; replaces default substring match. */
    matchSuggestion?: (filter: string, choice: UnknownValue) => boolean;
    /** Gate controlling whether the dropdown opens at all. */
    shouldRenderSuggestions?: (filter: string) => boolean;
    /** Label of a "(none)" entry; accepted for API parity but not rendered in the array variant. */
    emptyText?: string;
    /** Called with the current filter text on every keystroke. For server-side filtering outside ReferenceArrayInput. No debounce applied — caller decides. */
    setFilter?: (filter: string) => void;
  } & Pick<PopoverPrimitive.PopoverProps, "modal">;

const EMPTY_VALUES: UnknownValue[] = [];

function AutocompleteArrayInput(props: AutocompleteArrayInputProps) {
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
    clearOnBlur = false,
    openOnFocus: _openOnFocus = false, // array variant always opens on focus; prop kept for API parity
    selectOnFocus = false,
    handleHomeEndKeys = false,
    isOptionEqualToValue,
    matchSuggestion,
    shouldRenderSuggestions,
    setFilter,
    // emptyText: accepted for API parity; not rendered in array variant
    emptyText: _emptyText,
  } = props;
  const {
    allChoices = [],
    source,
    resource,
    isFromReference,
    setFilters,
  } = useChoicesContext(props);
  const {
    id,
    field,
    fieldState,
    isRequired: _isRequired,
  } = useInput({ ...props, source });
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

  // `field.value` may be `undefined` (no default + uncontrolled state).
  // Coalesce to a stable empty array so the rest of the component can rely on
  // array semantics without crashing or invalidating memoized deps.
  const values: UnknownValue[] = Array.isArray(field.value)
    ? field.value
    : EMPTY_VALUES;

  const inputRef = React.useRef<HTMLInputElement>(null);
  const listRef = React.useRef<HTMLDivElement>(null);
  const anchorRef = React.useRef<HTMLDivElement>(null);
  const [open, setOpen] = React.useState(false);

  // Keep stable references for the debounced filter setter so it isn't
  // recreated each render (which would defeat debouncing).
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
        setFiltersRef.current(
          filterToQueryRef.current(filter),
          undefined,
          true,
        );
      }, debounceDelay),
    [debounceDelay],
  );

  useEffect(() => () => debouncedSetFilters.cancel(), [debouncedSetFilters]);

  const [filterValue, setFilterValue] = React.useState("");

  // Prop 10: Reset filter when field.value changes externally (e.g. form reset).
  // biome-ignore lint/correctness/useExhaustiveDependencies: field.value is the intentional trigger (reset on external value change); the body only cancels/clears and does not read it.
  useEffect(() => {
    debouncedSetFilters.cancel();
    setFilterValue("");
  }, [field.value, debouncedSetFilters]);

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

  const handleUnselect = useEvent((choice: UnknownValue) => {
    field.onChange(
      values.filter((v: UnknownValue) => !isEqual(getChoiceValue(choice), v)),
    );
    field.onBlur();
  });

  const handleKeyDown = useEvent((e: React.KeyboardEvent<HTMLDivElement>) => {
    const input = inputRef.current;
    if (input) {
      if (e.key === "Delete" || e.key === "Backspace") {
        if (input.value === "") {
          field.onChange(values.slice(0, -1));
        }
      }
      // This is not a default behavior of the <input /> field
      if (e.key === "Escape") {
        input.blur();
      }
    }
  });

  // Base unselected choices (not yet in field.value).
  const unselectedChoices = allChoices.filter(
    (choice) =>
      !values.some((v: UnknownValue) => isEqual(getChoiceValue(choice), v)),
  );
  const selectedChoices = allChoices.filter((choice) =>
    values.some((v: UnknownValue) => isEqual(getChoiceValue(choice), v)),
  );

  // Prop 1: limitChoicesToValue — show only selected choices in the dropdown.
  let availableChoices = limitChoicesToValue
    ? selectedChoices
    : unselectedChoices;

  // matchSuggestion: when provided, apply custom filtering for non-reference inputs.
  if (matchSuggestion && !isFromReference && filterValue !== "") {
    availableChoices = availableChoices.filter((choice) =>
      matchSuggestion(filterValue, choice),
    );
  }

  // Prop 2: suggestionLimit — cap after filtering.
  if (suggestionLimit !== undefined) {
    availableChoices = availableChoices.slice(0, suggestionLimit);
  }

  // shouldRenderSuggestions: gate on the effective open state.
  const effectiveOpen =
    open && (!shouldRenderSuggestions || shouldRenderSuggestions(filterValue));

  // Prop 3: noOptionsText — fall back to translated string.
  const emptyText =
    noOptionsText ??
    translate("ra.navigation.no_results", { _: "No matching item found." });

  const getInputText = useCallback(
    (selectedChoice: UnknownValue) => {
      if (typeof inputText === "function") {
        return inputText(selectedChoice);
      }
      return getChoiceText(selectedChoice);
    },
    [inputText, getChoiceText],
  );

  // Prop 8: handleHomeEndKeys for the inline CommandPrimitive.Input.
  const handleInputKeyDown = useCallback(
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

  // Prop 6 + 7: openOnFocus / selectOnFocus.
  // The array variant always opens on focus (existing behavior); openOnFocus is
  // a no-op override since open-on-focus is the default for the inline input.
  const handleFocus = useCallback(
    (e: React.FocusEvent<HTMLInputElement>) => {
      setOpen(true);
      if (selectOnFocus) e.currentTarget.select();
    },
    [selectOnFocus],
  );

  // Prop 5: clearOnBlur.
  const handleBlur = useCallback(() => {
    setOpen(false);
    if (clearOnBlur) setFilterValue("");
  }, [clearOnBlur]);

  // handleChange used by useSupportCreateSuggestion: appends the chosen value to the array.
  const handleChange = useCallback(
    async (val: UnknownValue) => {
      setFilterValue("");
      if (isFromReference) {
        debouncedSetFilters.cancel();
        setFilters(filterToQuery(""));
      }
      field.onChange([...values, val]);
      field.onBlur();
    },
    [
      field,
      values,
      isFromReference,
      debouncedSetFilters,
      setFilters,
      filterToQuery,
    ],
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

  // Append createItem after the suggestionLimit slice so it is always visible.
  const finalChoices = createItem
    ? [...availableChoices, createItem]
    : availableChoices;

  return (
    <>
      <Field className={props.className} data-invalid={invalid || undefined}>
        {props.label !== false && (
          <FieldLabel htmlFor={id}>
            <FieldTitle
              label={props.label}
              source={props.source ?? source}
              resource={resource}
              isRequired={_isRequired}
            />
          </FieldLabel>
        )}
        <Command
          onKeyDown={handleKeyDown}
          shouldFilter={!isFromReference && !matchSuggestion}
          className="overflow-visible bg-transparent"
        >
          <Popover
            open={effectiveOpen}
            onOpenChange={(isOpen) => {
              if (!isOpen) setOpen(false);
            }}
            modal={modal}
          >
            <PopoverAnchor asChild>
              <div
                ref={anchorRef}
                className="group rounded-md bg-transparent dark:bg-input/30 border border-input px-3 py-1.75 text-sm transition-all ring-offset-background focus-within:border-ring focus-within:ring-ring/50 focus-within:ring-[3px]"
              >
                <div className="flex flex-wrap gap-1">
                  {selectedChoices.map((choice) => (
                    <Badge key={getChoiceValue(choice)} variant="outline">
                      {getInputText(choice)}
                      <button
                        type="button"
                        className="ml-1 rounded-full outline-none ring-offset-background focus:ring-2 focus:ring-ring focus:ring-offset-2"
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            handleUnselect(choice);
                          }
                        }}
                        onMouseDown={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                        }}
                        onClick={(e) => {
                          e.preventDefault();
                          handleUnselect(choice);
                        }}
                      >
                        <span className="sr-only">
                          {translate("ra.action.remove", {
                            _: "Remove",
                          })}
                        </span>
                        <X className="size-3" />
                      </button>
                    </Badge>
                  ))}
                  {/* Avoid having the "Search" Icon by not using CommandInput */}
                  <CommandPrimitive.Input
                    ref={inputRef}
                    id={id}
                    aria-invalid={invalid || undefined}
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
                    onBlur={handleBlur}
                    onFocus={handleFocus}
                    onKeyDown={handleInputKeyDown}
                    placeholder={placeholder}
                    className="ml-2 flex-1 bg-transparent outline-none placeholder:text-muted-foreground"
                  />
                </div>
              </div>
            </PopoverAnchor>
            <PopoverContent
              style={{ width: "var(--radix-popover-trigger-width)" }}
              className="p-0"
              onOpenAutoFocus={(e) => e.preventDefault()}
              onCloseAutoFocus={(e) => e.preventDefault()}
              onInteractOutside={(e) => {
                if (anchorRef.current?.contains(e.target as Node)) {
                  e.preventDefault();
                }
              }}
            >
              <CommandList ref={listRef}>
                {finalChoices.length > 0 ? (
                  <CommandGroup>
                    {finalChoices.map((choice) => {
                      const isCreateItem =
                        !!createItem && choice?.id === createItem.id;
                      const choiceText = getChoiceText(
                        isCreateItem ? createItem : choice,
                      );
                      return (
                        <CommandItem
                          key={
                            isCreateItem ? "__create__" : getChoiceValue(choice)
                          }
                          value={
                            isCreateItem
                              ? `?${filterValue}?`
                              : getChoiceValue(choice)
                          }
                          keywords={
                            isCreateItem || React.isValidElement(choiceText)
                              ? undefined
                              : [choiceText]
                          }
                          onMouseDown={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                          }}
                          onSelect={() => {
                            if (isCreateItem) {
                              handleChangeWithCreateSupport(createItem.value);
                            } else {
                              setFilterValue("");
                              if (isFromReference) {
                                debouncedSetFilters.cancel();
                                setFilters(filterToQuery(""));
                              }
                              field.onChange([
                                ...values,
                                getChoiceValue(choice),
                              ]);
                              field.onBlur();
                            }
                          }}
                          className="cursor-pointer"
                        >
                          {choiceText}
                        </CommandItem>
                      );
                    })}
                  </CommandGroup>
                ) : (
                  <div className="px-3 py-2 text-sm text-muted-foreground">
                    {emptyText}
                  </div>
                )}
              </CommandList>
            </PopoverContent>
          </Popover>
        </Command>
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

export { AutocompleteArrayInput, type AutocompleteArrayInputProps };
