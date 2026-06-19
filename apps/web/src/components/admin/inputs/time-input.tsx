import * as React from "react";
import clsx from "clsx";
import type { InputProps } from "shadmin-core";
import { useInput, FieldTitle, ValidationError } from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { InputHelperText } from "@/components/admin/common/input-helper-text";

/**
 * Time picker input for editing time values in `hh:mm` format.
 *
 * Use `<TimeInput>` for time-of-day fields like opening hours, alarms, or scheduling slots.
 * Renders a native browser time picker. The value can be a Date object or a string.
 * Returns a string formatted as "HH:mm" by default.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/time-input TimeInput documentation}
 * @see {@link https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/time MDN documentation for input type="time"}
 *
 * @example
 * import { Edit, SimpleForm, TimeInput, TextInput } from '@/components/admin';
 *
 * const EventEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <TimeInput source="opens_at" />
 *       <TimeInput source="closes_at" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function TimeInput({
  className,
  defaultValue,
  format = formatTime,
  parse = parseTime,
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
}: TimeInputProps) {
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
  const localInputRef = React.useRef<HTMLInputElement>(undefined);
  // TimeInput is not really a controlled input to ensure users can start entering a time, go to another input and come back to complete it.
  // This ref stores the value that is passed to the input defaultValue prop to solve this issue.
  const initialDefaultValueRef = React.useRef(field.value);
  // As the defaultValue prop won't trigger a remount of the HTML input, we will force it by changing the key.
  const [inputKey, setInputKey] = React.useState(1);
  // This ref let us track that the last change of the form state value was made by the input itself
  const wasLastChangedByInput = React.useRef(false);

  // This effect ensures we stay in sync with the react-hook-form state when the value changes from outside the input.
  React.useEffect(() => {
    if (wasLastChangedByInput.current) {
      wasLastChangedByInput.current = false;
      return;
    }

    const hasNewValueFromForm =
      localInputRef.current?.value !== field.value &&
      !(localInputRef.current?.value === "" && field.value == null);

    if (hasNewValueFromForm) {
      initialDefaultValueRef.current = field.value;
      setInputKey((r) => r + 1);
      wasLastChangedByInput.current = false;
    }
  }, [field.value]);

  const { onBlur: onBlurFromField } = field;
  const hasFocus = React.useRef(false);

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
    // Time strings without seconds are valid: "HH:mm". Browser may also include "HH:mm:ss".
    const isNewValueValid = newValue === "" || timeRegex.test(newValue);

    if (newValue !== "" && newValue != null && isNewValueValid) {
      field.onChange(newValue);
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
    const isNewValueValid = newValue === "" || timeRegex.test(newValue);

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
        type="time"
        onChange={handleChange}
        onFocus={handleFocus}
        onBlur={handleBlur}
        className={clsx(
          "ra-input",
          `ra-input-${source}`,
          "scheme-light dark:scheme-dark relative [&::-webkit-calendar-picker-indicator]:cursor-pointer [&::-webkit-calendar-picker-indicator]:absolute [&::-webkit-calendar-picker-indicator]:right-3 [&::-webkit-calendar-picker-indicator]:opacity-100 appearance-none",
          className,
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

type TimeInputProps = Omit<InputProps, "defaultValue"> & {
  defaultValue?: string | number | Date;
} & Omit<React.ComponentProps<"input">, "defaultValue" | "type">;

export { TimeInput, type TimeInputProps };

const leftPad =
  (nb = 2) =>
  (value: number) =>
    ("0".repeat(nb) + value).slice(-nb);
const leftPad2 = leftPad(2);

/**
 * @param {Date} value value to convert
 * @returns {String} A standardized time (hh:mm), to be passed to an <input type="time" />
 */
const convertDateToString = (value: Date) => {
  if (!(value instanceof Date) || Number.isNaN(value.getDate())) return "";
  const hh = leftPad2(value.getHours());
  const mm = leftPad2(value.getMinutes());
  return `${hh}:${mm}`;
};

// hh:mm or hh:mm:ss
const timeRegex = /^\d{2}:\d{2}(:\d{2})?$/;

/**
 * Converts a date/time from the dataProvider to a time string in the "hh:mm" format,
 * suitable for an <input type="time" />.
 *
 * @param {Date | String} value date string or object
 */
const formatTime = (value: string | Date | null | undefined) => {
  if (value == null || value === "") {
    return "";
  }

  if (value instanceof Date) {
    return convertDateToString(value);
  }
  // Valid time strings should not be converted
  if (typeof value === "string" && timeRegex.test(value)) {
    return value;
  }

  // Otherwise try to parse as a Date
  return convertDateToString(new Date(value));
};

/**
 * Converts a "hh:mm" or "hh:mm:ss" string entered using a <input type="time" /> input
 * into a Date object (today's date at that local time), for dataProvider compatibility.
 * Returns null for empty values.
 */
const parseTime = (value: string): Date | null => {
  if (value == null || value === "") {
    return null;
  }
  if (timeRegex.test(value)) {
    const [hh, mm, ss = "0"] = value.split(":");
    const d = new Date();
    d.setHours(parseInt(hh, 10), parseInt(mm, 10), parseInt(ss, 10), 0);
    return d;
  }
  return null;
};

/**
 * Merges multiple refs into a single callback ref.
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
    // biome-ignore lint/correctness/useExhaustiveDependencies: variadic refs array is intentionally spread as the dependency list (useForkRef pattern); Biome cannot statically verify a non-literal.
  }, refs);
}
