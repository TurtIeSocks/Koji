import type { InputProps } from "shadmin-core";
import {
  FieldTitle,
  useInput,
  useResourceContext,
  useTranslate,
  ValidationError,
} from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { cn } from "@/lib/utils";

const getBooleanFromString = (value: string): boolean | null => {
  if (value === "true") return true;
  if (value === "false") return false;
  return null;
};

const getStringFromBoolean = (value?: boolean | null): string => {
  if (value === true) return "true";
  if (value === false) return "false";
  return "";
};

// Radix Select doesn't accept "" as a SelectItem value, so we use a sentinel
// string for the "null" option and map back to "" / null when reading/writing.
const NULL_OPTION = "__null__";

type NullableBooleanInputProps = InputProps & {
  className?: string;
  nullLabel?: string;
  trueLabel?: string;
  falseLabel?: string;
};

/**
 * Three-option Select input for boolean values that can be `null`.
 *
 * Use `<NullableBooleanInput>` when the field can be `true`, `false`, or `null` (unknown).
 * Renders a dropdown with three options (default labels: "", "Yes", "No"). The labels can
 * be customized via `nullLabel`, `trueLabel`, and `falseLabel` props.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/nullable-boolean-input NullableBooleanInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, NullableBooleanInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <NullableBooleanInput source="is_published" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function NullableBooleanInput(props: NullableBooleanInputProps) {
  const {
    className,
    format = getStringFromBoolean,
    parse = getBooleanFromString,
    helperText,
    label,
    onBlur,
    onChange,
    resource: resourceProp,
    disabled,
    readOnly,
    source: sourceProp,
    validate,
    nullLabel,
    trueLabel,
    falseLabel,
    ...rest
  } = props;
  const resource = useResourceContext({ resource: resourceProp });
  const translate = useTranslate();

  const { id, field, fieldState, isRequired } = useInput({
    format,
    parse,
    onBlur,
    onChange,
    resource,
    source: sourceProp!,
    validate,
    disabled,
    readOnly,
    ...rest,
  });

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  // field.value will be "" / "true" / "false" thanks to format. Map "" to sentinel.
  const selectValue =
    field.value === "" || field.value == null ? NULL_OPTION : field.value;

  const handleValueChange = (value: string) => {
    const next = value === NULL_OPTION ? "" : value;
    field.onChange(next);
  };

  return (
    <Field
      data-invalid={invalid || undefined}
      className={cn("w-full min-w-20", className)}
    >
      {label !== false && label !== "" && (
        <FieldLabel htmlFor={id}>
          <FieldTitle
            label={label}
            source={sourceProp}
            resource={resource}
            isRequired={isRequired}
          />
        </FieldLabel>
      )}
      <Select
        // Key based on value: avoids Radix issue where onValueChange fires
        // with empty string when the controlled value changes externally.
        key={`nullable-boolean:${selectValue}`}
        value={selectValue}
        onValueChange={handleValueChange}
        disabled={disabled || readOnly}
      >
        <SelectTrigger
          id={id}
          className="w-full transition-all hover:bg-accent"
          aria-invalid={invalid || undefined}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent position="popper">
          <SelectItem value={NULL_OPTION}>
            {nullLabel
              ? translate(nullLabel, { _: nullLabel })
              : translate("ra.boolean.null", { _: "" })}
          </SelectItem>
          <SelectItem value="false">
            {falseLabel
              ? translate(falseLabel, { _: falseLabel })
              : translate("ra.boolean.false", { _: "No" })}
          </SelectItem>
          <SelectItem value="true">
            {trueLabel
              ? translate(trueLabel, { _: trueLabel })
              : translate("ra.boolean.true", { _: "Yes" })}
          </SelectItem>
        </SelectContent>
      </Select>
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </Field>
  );
}

export { NullableBooleanInput, type NullableBooleanInputProps };
