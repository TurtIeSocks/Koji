import { X } from "lucide-react";
import type { InputProps } from "shadmin-core";
import {
  useInput,
  useResourceContext,
  FieldTitle,
  useTranslate,
  ValidationError,
} from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { sanitizeInputRestProps } from "@/lib/sanitize-input-rest-props";
import { cn } from "@/lib/utils";

type TextInputProps = InputProps & {
  multiline?: boolean;
  inputClassName?: string;
  resettable?: boolean;
} & React.ComponentProps<"textarea"> &
  React.ComponentProps<"input">;

/**
 * Single-line or multiline text input for string values.
 *
 * Use `<TextInput>` for short text fields like titles or names. Set `multiline` to `true`
 * for longer content like descriptions or comments. Wraps shadcn's `<Input>` or `<Textarea>`
 * component depending on the `multiline` prop.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/text-input TextInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, TextInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <TextInput source="description" multiline rows={4} />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function TextInput(props: TextInputProps) {
  const resource = useResourceContext(props);
  const {
    label,
    source,
    multiline,
    className,
    inputClassName,
    helperText,
    resettable = false,
    type = "text",
    ...rest
  } = props;
  const { id, field, fieldState, isRequired } = useInput({ ...props, type });
  const translate = useTranslate();

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;
  const hasValue = field.value != null && field.value !== "";

  const handleReset = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    field.onChange("");
  };

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
      {multiline ? (
        <Textarea
          {...sanitizeInputRestProps(rest)}
          {...field}
          id={id}
          aria-invalid={invalid || undefined}
          className={inputClassName}
        />
      ) : resettable ? (
        <div className="relative flex items-center">
          <Input
            {...sanitizeInputRestProps(rest)}
            {...field}
            id={id}
            type={type}
            aria-invalid={invalid || undefined}
            className={cn(inputClassName, hasValue && "pr-8")}
          />
          {hasValue && (
            <button
              type="button"
              aria-label={translate("ra.action.clear_input_value", {
                _: "Clear",
              })}
              className="absolute right-2 p-0 text-muted-foreground opacity-50 hover:opacity-100 hover:bg-transparent"
              onClick={handleReset}
              tabIndex={-1}
            >
              <X className="size-4" />
            </button>
          )}
        </div>
      ) : (
        <Input
          {...sanitizeInputRestProps(rest)}
          {...field}
          id={id}
          type={type}
          aria-invalid={invalid || undefined}
          className={inputClassName}
        />
      )}
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>
    </Field>
  );
}

export { TextInput, type TextInputProps };
