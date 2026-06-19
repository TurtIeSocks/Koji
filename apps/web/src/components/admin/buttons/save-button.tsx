import * as React from "react";
import type { MouseEventHandler } from "react";
import { useCallback } from "react";
import type {
  CreateParams,
  RaRecord,
  TransformData,
  UpdateParams,
} from "shadmin-core";
import {
  setSubmissionErrors,
  useSaveContext,
  useTranslate,
  warning,
} from "shadmin-core";
import { Save } from "lucide-react";
import { useFormContext, useFormState } from "react-hook-form";
import type { UseMutationOptions } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import type { UnknownRecord, UnknownValue } from "@/lib/unknown-types";

/**
 * A button that saves form data with loading state and validation.
 *
 * Automatically handles form submission, validation, and loading states. Shows a spinner during
 * save operations and can be disabled when the form is pristine or invalid.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/save-button SaveButton documentation}
 *
 * @example
 * import { SimpleForm, SaveButton } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm toolbar={<SaveButton />}>
 *       // form inputs here
 *     </SimpleForm>
 *   </Edit>
 * )
 */
function SaveButton<RecordType extends RaRecord = RaRecord>(
  props: SaveButtonProps<RecordType>,
) {
  const {
    className,
    icon = defaultIcon,
    label = "ra.action.save",
    onClick,
    mutationOptions,
    alwaysEnable = false,
    disabled: disabledProp,
    type = "submit",
    transform,
    variant = "default",
    ref,
    ...rest
  } = props;
  const translate = useTranslate();
  const form = useFormContext();
  const saveContext = useSaveContext();
  const { isValidating, isSubmitting, disabled: formDisabled } = useFormState();
  const disabled = alwaysEnable
    ? disabledProp
    : disabledProp || isValidating || isSubmitting || formDisabled;

  warning(
    type === "submit" &&
      ((mutationOptions &&
        (mutationOptions.onSuccess || mutationOptions.onError)) ||
        transform),
    'Cannot use <SaveButton mutationOptions> props on a button of type "submit". To override the default mutation options on a particular save button, set the <SaveButton type="button"> prop, or set mutationOptions in the main view component (<Create> or <Edit>).',
  );

  const handleSubmit = useCallback(
    async (values: UnknownRecord) => {
      let errors: unknown;
      if (saveContext?.save) {
        errors = await saveContext.save(values, {
          ...mutationOptions,
          transform,
        });
      }
      if (errors != null) {
        setSubmissionErrors(errors, form.setError);
      }
    },
    [form.setError, saveContext, mutationOptions, transform],
  );

  const handleClick: MouseEventHandler<HTMLButtonElement> = useCallback(
    async (event) => {
      if (onClick) {
        onClick(event);
      }
      if (event.defaultPrevented) {
        return;
      }
      if (type === "button") {
        // this button doesn't submit the form, so it doesn't trigger useIsFormInvalid in <FormContent>
        // therefore we need to check for errors manually
        event.stopPropagation();
        await form.handleSubmit(handleSubmit)(event);
      }
    },
    [onClick, type, form, handleSubmit],
  );

  const displayedLabel = label && translate(label, { _: label });

  return (
    <Button
      ref={ref}
      variant={variant}
      type={type}
      disabled={disabled}
      onClick={handleClick}
      aria-label={
        typeof displayedLabel === "string" ? displayedLabel : undefined
      }
      className={className}
      {...rest}
    >
      {isSubmitting ? <Spinner /> : icon}
      {displayedLabel}
    </Button>
  );
}

const defaultIcon = <Save />;

interface Props<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> {
  className?: string;
  alwaysEnable?: boolean;
  disabled?: boolean;
  icon?: React.ReactNode;
  label?: string;
  mutationOptions?: UseMutationOptions<
    RecordType,
    MutationOptionsError,
    CreateParams<RecordType> | UpdateParams<RecordType>
  >;
  transform?: TransformData;
  variant?:
    | "default"
    | "destructive"
    | "outline"
    | "secondary"
    | "ghost"
    | "link";
}

type SaveButtonProps<RecordType extends RaRecord = RaRecord> =
  Props<RecordType> & React.ComponentProps<"button">;

export { SaveButton, type SaveButtonProps };
