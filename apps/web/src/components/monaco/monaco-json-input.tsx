import type { InputProps } from "shadmin-core";
import {
  FieldTitle,
  useInput,
  useResourceContext,
  ValidationError,
} from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import Editor from "@monaco-editor/react";

interface MonacoJsonInputProps extends InputProps {
  height?: number | string;
  schema?: object;
  readOnly?: boolean;
  className?: string;
}

/**
 * JSON editor input powered by Monaco (VS Code's editor engine).
 *
 * Parses the editor content on every change — if the JSON is valid, the
 * parsed value is written to the form; otherwise the raw string is stored so
 * the user can keep editing without losing work. Supports optional JSON Schema
 * validation via the `schema` prop.
 *
 * @example
 * import { MonacoJsonInput } from '@/components/monaco';
 *
 * <MonacoJsonInput source="args" height={400}
 *   schema={{ type: "object", properties: { key: { type: "string" } } }} />
 */
function MonacoJsonInput(props: MonacoJsonInputProps) {
  const {
    label,
    source,
    resource: resourceProp,
    helperText,
    height = 300,
    schema,
    readOnly,
    className,
  } = props;
  const resource = useResourceContext({ resource: resourceProp });
  const { id, field, fieldState, isRequired } = useInput(props);
  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  const stringValue =
    typeof field.value === "string"
      ? field.value
      : field.value != null
        ? JSON.stringify(field.value, null, 2)
        : "";

  const handleChange = (val: string | undefined) => {
    const s = val ?? "";
    try {
      field.onChange(JSON.parse(s));
    } catch {
      field.onChange(s);
    }
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
      <div className="overflow-hidden rounded-md border" style={{ height }}>
        <Editor
          height={height}
          defaultLanguage="json"
          value={stringValue}
          onChange={handleChange}
          options={{
            readOnly: readOnly ?? false,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
          }}
          beforeMount={(monaco) => {
            if (schema) {
              monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
                validate: true,
                schemas: [
                  { uri: "https://koji/schema", fileMatch: ["*"], schema },
                ],
              });
            }
          }}
        />
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

export { MonacoJsonInput, type MonacoJsonInputProps };
