import * as React from "react";
import { Fragment, useState, type Ref } from "react";
import { Trash } from "lucide-react";
import { Button } from "@/components/ui/button";
import { humanize, singularize } from "inflection";
import type {
  MutationMode,
  RedirectionSideEffect,
  UseDeleteOptions,
} from "shadmin-core";
import {
  useCanAccess,
  useDeleteController,
  useDeleteWithUndoController,
  useGetRecordRepresentation,
  useNotify,
  useRecordContext,
  useRedirect,
  useResourceContext,
  useResourceTranslation,
  useTranslate,
  useUnselect,
} from "shadmin-core";
import { Confirm } from "@/components/admin/feedback/confirm";

type DeleteButtonProps = {
  label?: string;
  size?: "default" | "sm" | "lg" | "icon";
  onClick?: React.ReactEventHandler<HTMLButtonElement>;
  mutationMode?: MutationMode;
  mutationOptions?: UseDeleteOptions;
  redirect?: RedirectionSideEffect;
  resource?: string;
  successMessage?: string;
  className?: string;
  ref?: Ref<HTMLButtonElement>;
  variant?:
    | "default"
    | "destructive"
    | "outline"
    | "secondary"
    | "ghost"
    | "link";
};

/**
 * A button that deletes a record, with undoable (default), optimistic, or pessimistic strategies.
 *
 * Defaults to `mutationMode="undoable"` — the delete is queued and a notification with an undo
 * button is shown for a few seconds. Set `mutationMode="pessimistic"` (or use
 * `<DeleteWithConfirmButton>`) to ask for confirmation before firing the mutation.
 *
 * Automatically redirects after deletion and reads the resource and record from context.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/delete-button DeleteButton documentation}
 *
 * @example
 * import { DeleteButton, Edit } from '@/components/admin';
 *
 * const PostEdit = () => (
 *     <Edit actions={<DeleteButton />}>
 *         ...
 *     </Edit>
 * );
 */
function DeleteButton(props: DeleteButtonProps) {
  const { mutationMode = "undoable" } = props;
  if (mutationMode === "undoable") {
    return <DeleteWithUndoButton {...props} />;
  }
  return <DeleteWithConfirmButton {...props} mutationMode={mutationMode} />;
}

const defaultDestructiveClassName =
  "cursor-pointer hover:bg-destructive/10! text-destructive! border-destructive! focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40";

/**
 * Delete button variant that fires immediately and shows an undo notification.
 *
 * Equivalent to `<DeleteButton mutationMode="undoable">`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/delete-button DeleteButton documentation}
 */
function DeleteWithUndoButton(props: Omit<DeleteButtonProps, "mutationMode">) {
  const {
    label: labelProp,
    onClick,
    ref,
    size,
    mutationOptions,
    redirect = "list",
    successMessage,
    variant = "outline",
    className = defaultDestructiveClassName,
  } = props;
  const record = useRecordContext(props);
  const resource = useResourceContext(props);
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "delete",
    resource,
    record,
  });

  const { isPending, handleDelete } = useDeleteWithUndoController({
    record,
    resource,
    redirect,
    onClick,
    mutationOptions,
    successMessage,
  });
  const translate = useTranslate();
  const getRecordRepresentation = useGetRecordRepresentation(resource);
  let recordRepresentation = getRecordRepresentation(record);
  const resourceName = translate(`resources.${resource}.forcedCaseName`, {
    smart_count: 1,
    _: humanize(
      translate(`resources.${resource}.name`, {
        smart_count: 1,
        _: resource ? singularize(resource) : undefined,
      }),
      true,
    ),
  });
  if (React.isValidElement(recordRepresentation)) {
    recordRepresentation = `#${record?.id}`;
  }
  const label = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.action.delete`,
    baseI18nKey: "ra.action.delete",
    options: {
      name: resourceName,
      recordRepresentation,
    },
    userText: labelProp,
  });

  if (isAccessPending || !canAccess) return null;
  return (
    <Button
      ref={ref}
      variant={variant}
      type="button"
      onClick={handleDelete}
      disabled={isPending}
      aria-label={typeof label === "string" ? label : undefined}
      size={size}
      className={className}
    >
      <Trash />
      {label}
    </Button>
  );
}

/**
 * Delete button variant that asks the user to confirm before firing the mutation.
 *
 * Equivalent to `<DeleteButton mutationMode="pessimistic">` (or `optimistic`).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/delete-button DeleteButton documentation}
 */
type DeleteWithConfirmButtonProps = Omit<DeleteButtonProps, "mutationMode"> & {
  mutationMode?: Exclude<MutationMode, "undoable">;
  confirmTitle?: React.ReactNode;
  confirmContent?: React.ReactNode;
  confirmColor?: "primary" | "warning";
  titleTranslateOptions?: object;
  contentTranslateOptions?: object;
};

function DeleteWithConfirmButton(props: DeleteWithConfirmButtonProps) {
  const {
    label: labelProp,
    onClick,
    ref,
    size,
    mutationMode = "pessimistic",
    mutationOptions,
    redirect: redirectTo = "list",
    successMessage,
    variant = "outline",
    className = defaultDestructiveClassName,
    confirmTitle: confirmTitleProp,
    confirmContent: confirmContentProp,
    confirmColor = "primary",
    titleTranslateOptions = emptyObject,
    contentTranslateOptions = emptyObject,
  } = props;
  const record = useRecordContext(props);
  const resource = useResourceContext(props);
  if (!resource) {
    throw new Error(
      "<DeleteWithConfirmButton> components should be used inside a <Resource> component or provided with a resource prop.",
    );
  }
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "delete",
    resource,
    record,
  });
  const translate = useTranslate();
  const notify = useNotify();
  const unselect = useUnselect(resource);
  const redirect = useRedirect();
  const [open, setOpen] = useState(false);

  const { onSuccess, onError, ...otherMutationOptions } = mutationOptions || {};

  const { isPending, handleDelete } = useDeleteController({
    record,
    redirect: redirectTo,
    mutationMode,
    mutationOptions: {
      ...otherMutationOptions,
      onSuccess: (...args) => {
        setOpen(false);
        if (onSuccess) {
          onSuccess(...args);
        } else {
          notify(
            successMessage ?? `resources.${resource}.notifications.deleted`,
            {
              type: "info",
              messageArgs: {
                smart_count: 1,
                _: translate("ra.notification.deleted", { smart_count: 1 }),
              },
              undoable: false,
            },
          );
          if (record) unselect([record.id]);
          redirect(redirectTo, resource);
        }
      },
      onError: (...args) => {
        setOpen(false);
        if (onError) {
          onError(...args);
        } else {
          const [error] = args;
          notify(
            typeof error === "string"
              ? error
              : (error as Error)?.message || "ra.notification.http_error",
            {
              type: "error",
              messageArgs: {
                _:
                  typeof error === "string"
                    ? error
                    : (error as Error)?.message
                      ? (error as Error).message
                      : undefined,
              },
            },
          );
        }
      },
    },
    resource,
    successMessage,
  });

  const handleDialogOpen: React.ReactEventHandler<HTMLButtonElement> = (
    event,
  ) => {
    event.stopPropagation();
    setOpen(true);
    if (onClick) {
      onClick(event);
    }
  };
  const handleDialogClose = () => {
    setOpen(false);
  };

  const getRecordRepresentation = useGetRecordRepresentation(resource);
  let recordRepresentation = getRecordRepresentation(record);
  const resourceName = translate(`resources.${resource}.forcedCaseName`, {
    smart_count: 1,
    _: humanize(
      translate(`resources.${resource}.name`, {
        smart_count: 1,
        _: resource ? singularize(resource) : undefined,
      }),
      true,
    ),
  });
  if (React.isValidElement(recordRepresentation)) {
    recordRepresentation = `#${record?.id}`;
  }
  const label = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.action.delete`,
    baseI18nKey: "ra.action.delete",
    options: {
      name: resourceName,
      recordRepresentation,
    },
    userText: labelProp,
  });
  const confirmTitle = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.message.delete_title`,
    baseI18nKey: "ra.message.delete_title",
    options: {
      recordRepresentation,
      name: resourceName,
      id: record?.id,
      ...titleTranslateOptions,
    },
    userText: confirmTitleProp,
  });
  const confirmContent = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.message.delete_content`,
    baseI18nKey: "ra.message.delete_content",
    options: {
      recordRepresentation,
      name: resourceName,
      id: record?.id,
      ...contentTranslateOptions,
    },
    userText: confirmContentProp,
  });

  if (isAccessPending || !canAccess) return null;
  return (
    <Fragment>
      <Button
        ref={ref}
        variant={variant}
        type="button"
        onClick={handleDialogOpen}
        disabled={isPending}
        aria-label={typeof label === "string" ? label : undefined}
        size={size}
        className={className}
      >
        <Trash />
        {label}
      </Button>
      <Confirm
        isOpen={open}
        loading={isPending}
        title={confirmTitle}
        content={confirmContent}
        confirmColor={confirmColor}
        onConfirm={handleDelete}
        onClose={handleDialogClose}
      />
    </Fragment>
  );
}

const emptyObject = {};

export {
  DeleteButton,
  DeleteWithUndoButton,
  DeleteWithConfirmButton,
  type DeleteButtonProps,
  type DeleteWithConfirmButtonProps,
};
