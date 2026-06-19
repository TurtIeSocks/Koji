import * as React from "react";
import { X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverAnchor,
} from "@/components/ui/popover";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import type { InputProps } from "shadmin-core";
import {
  useInput,
  useResourceContext,
  FieldTitle,
  useTranslate,
  ValidationError,
} from "shadmin-core";
import { InputHelperText } from "@/components/admin/common/input-helper-text";

type TagProps = { key: string | number; onDelete: () => void };

type TextArrayInputProps = InputProps & {
  className?: string;
  placeholder?: string;
  /** Autocomplete suggestions shown in a dropdown below the input. */
  options?: string[];
  /**
   * Custom tag renderer. When provided, replaces the default Badge-per-tag
   * rendering. `getTagProps(tag, index)` returns `{ key, onDelete }`.
   */
  renderTags?: (
    tags: string[],
    getTagProps: (tag: string, index: number) => TagProps,
  ) => React.ReactNode;
};

const emptyArray: string[] = [];

/**
 * Form input for editing an array of strings, like tags or email addresses.
 *
 * Renders a text input where values are displayed as removable badges.
 * Users type text and press Enter to add items, or press Backspace
 * when the input is empty to remove the last item.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/text-array-input TextArrayInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, TextArrayInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextArrayInput source="tags" />
 *       <TextArrayInput source="emails" placeholder="Add an email..." />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function TextArrayInput(props: TextArrayInputProps) {
  const {
    className,
    defaultValue,
    disabled,
    format,
    helperText,
    label,
    name,
    onBlur,
    onChange,
    options,
    parse,
    placeholder,
    readOnly,
    renderTags,
    source,
    validate,
    ...rest
  } = props;
  const resource = useResourceContext(props);
  const { id, field, fieldState, isRequired } = useInput({
    defaultValue,
    disabled,
    format: format ?? ((v) => v ?? emptyArray),
    name,
    onBlur,
    onChange,
    parse,
    readOnly,
    source,
    validate,
  });
  const translate = useTranslate();

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  const inputRef = React.useRef<HTMLInputElement>(null);
  const [inputValue, setInputValue] = React.useState("");
  const [suggestionsOpen, setSuggestionsOpen] = React.useState(false);

  const values: string[] = field.value ?? emptyArray;

  const filteredOptions = React.useMemo(() => {
    if (!options || !inputValue.trim()) return [];
    const lower = inputValue.toLowerCase();
    return options.filter(
      (opt) => opt.toLowerCase().includes(lower) && !values.includes(opt),
    );
  }, [options, inputValue, values]);

  const handleAddValue = (text: string) => {
    const trimmed = text.trim();
    if (trimmed) {
      field.onChange([...values, trimmed]);
    }
    setInputValue("");
    setSuggestionsOpen(false);
  };

  const handleRemoveValue = (index: number) => {
    field.onChange(values.filter((_, i) => i !== index));
  };

  const getTagProps = (tag: string, index: number): TagProps => ({
    key: `${tag}-${index}`,
    onDelete: () => handleRemoveValue(index),
  });

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (inputValue.trim()) {
        handleAddValue(inputValue);
      }
    } else if (
      e.key === "Backspace" &&
      inputValue === "" &&
      values.length > 0 &&
      !readOnly
    ) {
      field.onChange(values.slice(0, -1));
    } else if (e.key === "Escape") {
      setSuggestionsOpen(false);
      inputRef.current?.blur();
    }
  };

  const defaultTagRenderer = () =>
    values.map((value, index) => (
      // biome-ignore lint/suspicious/noArrayIndexKey: tag values are user-entered primitives that may duplicate; index keeps keys unique and matches handleRemoveValue(index)
      <Badge key={`${value}-${index}`} variant="outline">
        {value}
        <button
          type="button"
          className="ml-1 cursor-pointer rounded-full outline-none ring-offset-background focus:ring-2 focus:ring-ring focus:ring-offset-2"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              handleRemoveValue(index);
            }
          }}
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          onClick={(e) => {
            e.preventDefault();
            handleRemoveValue(index);
          }}
          disabled={disabled || readOnly}
        >
          <span className="sr-only">
            {translate("ra.action.remove", { _: "Remove" })}
          </span>
          <X className="size-3" />
        </button>
      </Badge>
    ));

  const showSuggestions =
    !!options && suggestionsOpen && filteredOptions.length > 0;

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
      <Popover open={showSuggestions} onOpenChange={setSuggestionsOpen}>
        <PopoverAnchor asChild>
          <div
            className="group rounded-md bg-background shadow-xs dark:bg-input/30 border border-input px-3 py-1.75 text-sm transition-all ring-offset-background focus-within:border-ring focus-within:ring-ring/50 focus-within:ring-[3px]"
            {...rest}
          >
            <div className="flex flex-wrap gap-1">
              {renderTags
                ? renderTags(values, getTagProps)
                : defaultTagRenderer()}
              <input
                ref={inputRef}
                id={id}
                aria-invalid={invalid || undefined}
                value={inputValue}
                onChange={(e) => {
                  setInputValue(e.target.value);
                  if (options) setSuggestionsOpen(true);
                }}
                onKeyDown={handleKeyDown}
                onBlur={() => {
                  if (inputValue.trim()) {
                    handleAddValue(inputValue);
                  }
                  field.onBlur?.();
                  // delay close so clicks on suggestions register first
                  setTimeout(() => setSuggestionsOpen(false), 150);
                }}
                placeholder={values.length === 0 ? placeholder : undefined}
                disabled={disabled}
                readOnly={readOnly}
                className="ml-2 flex-1 bg-transparent outline-none placeholder:text-muted-foreground"
              />
            </div>
          </div>
        </PopoverAnchor>
        {showSuggestions && (
          <PopoverContent
            className="p-0 w-(--radix-popover-trigger-width)"
            align="start"
            onOpenAutoFocus={(e) => e.preventDefault()}
          >
            <Command>
              <CommandList>
                <CommandEmpty>
                  {translate("ra.navigation.no_results", {
                    _: "No results",
                  })}
                </CommandEmpty>
                <CommandGroup>
                  {filteredOptions.map((opt) => (
                    <CommandItem
                      key={opt}
                      value={opt}
                      onSelect={() => handleAddValue(opt)}
                    >
                      {opt}
                    </CommandItem>
                  ))}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        )}
      </Popover>
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </Field>
  );
}

export { TextArrayInput, type TextArrayInputProps, type TagProps };
