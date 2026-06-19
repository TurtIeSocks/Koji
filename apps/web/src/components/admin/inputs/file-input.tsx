import * as React from "react";
import type {
  ComponentType,
  ReactElement,
  ReactNode,
  HTMLAttributes,
} from "react";
import { Children, isValidElement, useEffect } from "react";
import type { InputProps } from "shadmin-core";
import {
  FieldTitle,
  RecordContextProvider,
  shallowEqual,
  useInput,
  useTranslate,
  ValidationError,
} from "shadmin-core";
import type {
  DropzoneOptions,
  FileRejection,
  DropEvent,
  DropzoneInputProps,
} from "react-dropzone";
import { useDropzone } from "react-dropzone";
import { XCircle } from "lucide-react";

import { cn } from "@/lib/utils";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { Button } from "@/components/ui/button";
import type { UnknownValue } from "@/lib/unknown-types";

/**
 * File upload input with drag-and-drop support and preview capabilities.
 *
 * Use `<FileInput>` for document uploads, images, PDFs, CSV files, or any file attachment field.
 * Powered by react-dropzone with support for multiple files, file type restrictions (accept), and
 * size constraints. Pass a child component (typically `<FileField>`) to render file previews.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/file-input FileInput documentation}
 * @see {@link https://react-dropzone.js.org/ React Dropzone documentation}
 *
 * @example
 * import {
 *   Edit,
 *   SimpleForm,
 *   TextInput,
 *   FileInput,
 *   FileField,
 * } from '@/components/admin';
 *
 * const DocumentEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <FileInput source="attachments" multiple accept={{ 'image/*': [], 'application/pdf': [] }}>
 *         <FileField source="src" title="title" />
 *       </FileInput>
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function FileInput(props: FileInputProps) {
  const {
    alwaysOn,
    defaultValue,
    format,
    label,
    helperText,
    name: nameProp,
    onBlur: onBlurProp,
    onChange: onChangeProp,
    parse,
    resource,
    source,
    validate,
    readOnly,
    disabled,

    accept,
    maxSize,
    minSize,
    multiple = false,
    options = {},

    children,
    className,
    inputProps: inputPropsOptions,

    onRemove: onRemoveProp,
    validateFileRemoval,

    placeholder,
    labelMultiple = "ra.input.file.upload_several",
    labelSingle = "ra.input.file.upload_single",
    removeIcon,
    ...rest
  } = props;
  const { onDrop: onDropProp } = options;
  const translate = useTranslate();

  // turn a browser dropped file structure into expected structure
  const transformFile = (file: UnknownValue) => {
    if (!(file instanceof File)) {
      return file;
    }

    const preview = URL.createObjectURL(file);
    const transformedFile: TransformedFile = {
      rawFile: file,
      src: preview,
      title: file.name,
    };

    return transformedFile;
  };

  const transformFiles = (files: UnknownValue[]) => {
    if (!files) {
      return multiple ? [] : null;
    }

    if (Array.isArray(files)) {
      return files.map(transformFile);
    }

    return transformFile(files);
  };

  const {
    id,
    field: { onChange, onBlur, value, name },
    isRequired,
    fieldState,
  } = useInput({
    alwaysOn,
    defaultValue,
    format: format || transformFiles,
    label,
    helperText,
    name: nameProp,
    onBlur: onBlurProp,
    onChange: onChangeProp,
    parse: parse || transformFiles,
    resource,
    source,
    validate,
    readOnly,
    disabled,
  });
  const files = value ? (Array.isArray(value) ? value : [value]) : [];

  const invalid = fieldState.invalid;
  const errorMessage =
    fieldState.error?.root?.message ?? fieldState.error?.message;

  const onDrop = (
    newFiles: File[],
    rejectedFiles: FileRejection[],
    event: DropEvent,
  ) => {
    const updatedFiles = multiple ? [...files, ...newFiles] : [...newFiles];

    if (multiple) {
      onChange(updatedFiles);
      onBlur();
    } else {
      onChange(updatedFiles[0]);
      onBlur();
    }

    if (onDropProp) {
      onDropProp(newFiles, rejectedFiles, event);
    }
  };

  const onRemove = (file: File | TransformedFile) => async () => {
    if (validateFileRemoval) {
      try {
        await validateFileRemoval(file);
      } catch {
        return;
      }
    }

    if (multiple) {
      const filteredFiles = files.filter(
        (stateFile) => !shallowEqual(stateFile, file),
      );
      onChange(filteredFiles);
      onBlur();
    } else {
      onChange(null);
      onBlur();
    }

    if (onRemoveProp) {
      onRemoveProp(file);
    }
  };

  const childrenElement =
    children && isValidElement(Children.only(children))
      ? (Children.only(children) as ReactElement)
      : undefined;

  const { getRootProps, getInputProps } = useDropzone({
    accept,
    maxSize,
    minSize,
    multiple,
    disabled: disabled || readOnly,
    ...options,
    onDrop,
  });

  return (
    <Field
      className={cn("w-full", className)}
      data-invalid={invalid || undefined}
      {...rest}
    >
      <FieldLabel
        htmlFor={id}
        className={disabled || readOnly ? "cursor-default" : "cursor-pointer"}
      >
        <FieldTitle
          label={label}
          source={source}
          resource={resource}
          isRequired={isRequired}
        />
      </FieldLabel>

      <div
        {...getRootProps({
          className: cn(
            "border-2 border-dashed border-muted rounded-lg p-6 text-center transition-colors",
            "hover:border-sidebar-ring focus:outline-none",
            disabled || readOnly
              ? "bg-muted cursor-not-allowed"
              : "bg-muted text-muted-foreground cursor-pointer",
            invalid && "border-destructive",
          ),
        })}
      >
        <input
          id={id}
          name={name}
          aria-invalid={invalid || undefined}
          {...getInputProps({
            ...inputPropsOptions,
          })}
        />

        {placeholder ? (
          placeholder
        ) : multiple ? (
          <p className="text-sm">{translate(labelMultiple)}</p>
        ) : (
          <p className="text-sm">{translate(labelSingle)}</p>
        )}
      </div>

      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? (
          <ValidationError error={errorMessage} />
        ) : null}
      </FieldError>

      {children && (
        <div className="previews flex flex-col gap-1">
          {files.map((file, index: number) => (
            <FileInputPreview
              key={
                (file as { rawFile?: File })?.rawFile?.name ??
                (file as { src?: string })?.src ??
                index
              }
              file={file}
              onRemove={onRemove(file)}
              removeIcon={removeIcon}
            >
              <RecordContextProvider value={file}>
                {childrenElement}
              </RecordContextProvider>
            </FileInputPreview>
          ))}
        </div>
      )}
    </Field>
  );
}

type FileInputProps = Omit<InputProps, "type"> & {
  accept?: DropzoneOptions["accept"];
  className?: string;
  children?: ReactNode;
  labelMultiple?: string;
  labelSingle?: string;
  maxSize?: DropzoneOptions["maxSize"];
  minSize?: DropzoneOptions["minSize"];
  multiple?: DropzoneOptions["multiple"];
  options?: DropzoneOptions;
  onRemove?: (file: File | TransformedFile) => void;
  placeholder?: ReactNode;
  removeIcon?: ComponentType<{ className?: string }>;
  inputProps?: DropzoneInputProps & React.ComponentProps<"input">;
  validateFileRemoval?(
    file: File | TransformedFile,
  ): boolean | Promise<boolean>;
};

interface FileWithPreview extends File {
  preview?: string;
}

interface TransformedFile {
  rawFile: FileWithPreview;
  src: string;
  title: string;
}

/**
 * Preview container for uploaded files in `<FileInput>`, with a remove button.
 *
 * @internal
 */
function FileInputPreview(props: FileInputPreviewProps) {
  const {
    className,
    children,

    file,
    onRemove,
    removeIcon: RemoveIcon = XCircle,

    ...rest
  } = props;

  const translate = useTranslate();

  useEffect(() => {
    return () => {
      const preview = "rawFile" in file ? file.rawFile.preview : file.preview;

      if (preview) {
        window.URL.revokeObjectURL(preview);
      }
    };
  }, [file]);

  return (
    <div className={cn("flex flex-row gap-1 group", className)} {...rest}>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="size-6 rounded-full shadow-sm cursor-pointer opacity-0 group-hover:opacity-100 transition-opacity"
        onClick={onRemove}
        aria-label={translate("ra.action.delete")}
        title={translate("ra.action.delete")}
      >
        <RemoveIcon className="size-4" />
      </Button>
      {children}
    </div>
  );
}

interface FileInputPreviewProps extends HTMLAttributes<HTMLDivElement> {
  file: FileWithPreview | TransformedFile;
  onRemove: () => void;
  removeIcon?: React.ComponentType<{ className?: string }>;
}

export {
  FileInput,
  type FileInputProps,
  FileInputPreview,
  type FileInputPreviewProps,
  type TransformedFile,
};
