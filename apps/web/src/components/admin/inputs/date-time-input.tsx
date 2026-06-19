import * as React from "react";
import type { InputProps } from "shadmin-core";
import { useInput, FieldTitle, ValidationError } from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { cn } from "@/lib/utils";

/**
 * Date and time picker input for editing datetime values with timezone support.
 *
 * Use `<DateTimeInput>` for timestamps like "created at", "updated at", or scheduled events.
 * Renders a native browser datetime-local picker. Expects and returns ISO 8601 formatted strings
 * (e.g. '2025-11-17T10:10:32.390Z'), automatically converting other formats like Date objects or timestamps.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/date-time-input DateTimeInput documentation}
 * @see {@link https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/datetime-local MDN documentation for input type="datetime-local"}
 *
 * @example
 * import {
 *   Edit,
 *   SimpleForm,
 *   DateTimeInput,
 *   TextInput,
 * } from '@/components/admin';
 *
 * const EventEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <DateTimeInput source="starts_at" />
 *       <DateTimeInput source="ends_at" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function DateTimeInput({
  className,
  inputClassName,
  defaultValue,
  format = formatDateTime,
  parse = convertDateStringToISO,
  label,
  helperText,
  onBlur,
  onChange,
  onFocus,
  source,
  resource,
  validate,
  disabled,
  readOnly,
  ...rest
}: DateTimeInputProps) {
  const { field, fieldState, id, isRequired } = useInput({
    defaultValue,
    onBlur,
    resource,
    source,
    validate,
    disabled,
    readOnly,
    format,
    parse,
    ...rest,
  });

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;
  const localInputRef = React.useRef<HTMLInputElement | null>(null);
  // DateInput is not a really controlled input to ensure users can start entering a date, go to another input and come back to complete it.
  // This ref stores the value that is passed to the input defaultValue prop to solve this issue.
  const initialDefaultValueRef = React.useRef(field.value);
  // As the defaultValue prop won't trigger a remount of the HTML input, we will force it by changing the key.
  const [inputKey, setInputKey] = React.useState(1);
  // This ref let us track that the last change of the form state value was made by the input itself
  const wasLastChangedByInput = React.useRef(false);

  // This effect ensures we stays in sync with the react-hook-form state when the value changes from outside the input
  // for instance by using react-hook-form reset or setValue methods.
  React.useEffect(() => {
    // Ignore react-hook-form state changes if it came from the input itself
    if (wasLastChangedByInput.current) {
      // Resets the flag to ensure futures changes are handled
      wasLastChangedByInput.current = false;
      return;
    }

    const hasNewValueFromForm =
      localInputRef.current?.value !== field.value &&
      !(localInputRef.current?.value === "" && field.value == null);

    if (hasNewValueFromForm) {
      // The value has changed from outside the input, we update the input value
      initialDefaultValueRef.current = field.value;
      // Trigger a remount of the HTML input
      setInputKey((r) => r + 1);
      // Resets the flag to ensure futures changes are handled
      wasLastChangedByInput.current = false;
    }
  }, [field.value]);

  const { onBlur: onBlurFromField } = field;
  const hasFocus = React.useRef(false);

  // update the input text when the user types in the input
  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    if (onChange) {
      onChange(event);
    }
    if (
      typeof event.target === "undefined" ||
      typeof event.target.value === "undefined"
    ) {
      return;
    }
    const target = event.target;
    const newValue = target.value;
    const isNewValueValid =
      newValue === "" || !Number.isNaN(new Date(target.value).getTime());

    // Some browsers will return null for an invalid date
    // so we only change react-hook-form value if it's not null.
    // The input reset is handled in the onBlur event handler
    if (newValue !== "" && newValue != null && isNewValueValid) {
      field.onChange(newValue);
      // Track the fact that the next react-hook-form state change was triggered by the input itself
      wasLastChangedByInput.current = true;
    }
  };

  const handleFocus = (event: React.FocusEvent<HTMLInputElement>) => {
    if (onFocus) {
      onFocus(event);
    }
    hasFocus.current = true;
  };

  const handleBlur = () => {
    hasFocus.current = false;

    if (!localInputRef.current) {
      return;
    }

    const newValue = localInputRef.current.value;
    // To ensure users can clear the input, we check its value on blur
    // and submit it to react-hook-form
    const isNewValueValid =
      newValue === "" ||
      !Number.isNaN(new Date(localInputRef.current.value).getTime());

    if (isNewValueValid && field.value !== newValue) {
      field.onChange(newValue ?? "");
    }

    if (onBlurFromField) {
      onBlurFromField();
    }
  };

  const { ref, name } = field;
  const inputRef = useForkRef(ref, localInputRef);

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
      <Input
        ref={inputRef}
        name={name}
        defaultValue={format(initialDefaultValueRef.current)}
        key={inputKey}
        type="datetime-local"
        onChange={handleChange}
        onFocus={handleFocus}
        onBlur={handleBlur}
        className={cn(
          "ra-input",
          `ra-input-${source}`,
          "scheme-light dark:scheme-dark relative [&::-webkit-calendar-picker-indicator]:cursor-pointer [&::-webkit-calendar-picker-indicator]:absolute [&::-webkit-calendar-picker-indicator]:right-3 [&::-webkit-calendar-picker-indicator]:opacity-100 appearance-none",
          inputClassName,
        )}
        disabled={disabled || readOnly}
        readOnly={readOnly}
        id={id}
        aria-invalid={invalid || undefined}
      />
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </Field>
  );
}

type DateTimeInputProps = Omit<InputProps, "defaultValue"> & {
  defaultValue?: string | number | Date;
  inputClassName?: string;
} & Omit<React.ComponentProps<"input">, "defaultValue" | "type">;

const leftPad =
  (nb = 2) =>
  (value: number) =>
    ("0".repeat(nb) + value).slice(-nb);
const leftPad4 = leftPad(4);
const leftPad2 = leftPad(2);

/**
 * @param {Date} value value to convert
 * @returns {String} A standardized datetime (yyyy-MM-ddThh:mm), to be passed to an <input type="datetime-local" />
 */
const convertDateToString = (value: Date) => {
  if (!(value instanceof Date) || Number.isNaN(value.getDate())) return "";
  const yyyy = leftPad4(value.getFullYear());
  const MM = leftPad2(value.getMonth() + 1);
  const dd = leftPad2(value.getDate());
  const hh = leftPad2(value.getHours());
  const mm = leftPad2(value.getMinutes());
  return `${yyyy}-${MM}-${dd}T${hh}:${mm}`;
};

// yyyy-MM-ddThh:mm
const dateTimeRegex = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/;

/**
 * Converts a date from the dataProvider, with timezone, to a date string
 * without timezone for use in an <input type="datetime-local" />.
 *
 * @param {Date | String} value date string or object
 */
const formatDateTime = (value: string | Date) => {
  // null, undefined and empty string values should not go through convertDateToString
  // otherwise, it returns undefined and will make the input an uncontrolled one.
  if (value == null || value === "") {
    return "";
  }

  if (value instanceof Date) {
    return convertDateToString(value);
  }
  // valid dates should not be converted
  if (dateTimeRegex.test(value)) {
    return value;
  }

  return convertDateToString(new Date(value));
};

// converts a date string entered using a datetime-local input
// into an ISO date using the browser timezone
const convertDateStringToISO = (
  value: string | null | undefined,
): string | null => {
  if (value == null || value === "") return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date.toISOString();
};

/**
 * Imported from material-ui
 * Merges refs into a single memoized callback ref or `null`.
 *
 * ```tsx
 * const rootRef = React.useRef<Instance>(null);
 * const refFork = useForkRef(rootRef, props.ref);
 *
 * return (
 *   <Root {...props} ref={refFork} />
 * );
 * ```
 *
 * @param {Array<React.Ref<Instance> | undefined>} refs The ref array.
 * @returns {React.RefCallback<Instance> | null} The new ref callback.
 */
function useForkRef<Instance>(
  ...refs: Array<React.Ref<Instance> | undefined>
): React.RefCallback<Instance> | null {
  const cleanupRef = React.useRef<() => void>(undefined);

  const refEffect = React.useCallback((instance: Instance) => {
    const cleanups = refs.map((ref) => {
      if (ref == null) {
        return null;
      }

      if (typeof ref === "function") {
        const refCallback = ref;
        // biome-ignore lint/suspicious/noConfusingVoidType: a ref callback legitimately returns void or a cleanup fn; narrowing void→undefined breaks the assignment from refCallback().
        const refCleanup: void | (() => void) = refCallback(instance);
        return typeof refCleanup === "function"
          ? refCleanup
          : () => {
              refCallback(null);
            };
      }

      ref.current = instance;
      return () => {
        ref.current = null;
      };
    });

    return () => {
      cleanups.forEach((refCleanup) => refCleanup?.());
    };
    // biome-ignore lint/correctness/useExhaustiveDependencies: variadic refs array is intentionally spread as the dependency list (useForkRef pattern) so the effect re-runs when any individual ref changes; Biome cannot statically verify a non-literal.
  }, refs);

  return React.useMemo(() => {
    if (refs.every((ref) => ref == null)) {
      return null;
    }

    return (value) => {
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = undefined;
      }

      if (value != null) {
        cleanupRef.current = refEffect(value);
      }
    };
    // TODO: uncomment once we enable eslint-plugin-react-compiler -- intentionally ignoring that the dependency array must be an array literal
    // biome-ignore lint/correctness/useExhaustiveDependencies: see TODO above -- variadic refs array is intentionally the non-literal dependency list (useForkRef pattern); Biome cannot statically verify it.
  }, refs);
}

export { DateTimeInput, type DateTimeInputProps };
