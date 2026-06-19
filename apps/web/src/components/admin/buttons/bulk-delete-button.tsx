import * as React from "react";
import { Fragment, useState, type Ref } from "react";
import { humanize, inflect } from "inflection";
import { Trash } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { RaRecord, UseBulkDeleteControllerParams } from "shadmin-core";
import {
  useCanAccess,
  useBulkDeleteController,
  useGetResourceLabel,
  useListContext,
  useResourceContext,
  useResourceTranslation,
  useTranslate,
} from "shadmin-core";
import { cn } from "@/lib/utils";
import type { ReactNode } from "react";
import { Confirm } from "@/components/admin/feedback/confirm";
import type { UnknownValue } from "@/lib/unknown-types";

type BulkDeleteButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> = {
  label?: string;
  icon?: ReactNode;
  ref?: Ref<HTMLButtonElement>;
} & React.ComponentPropsWithoutRef<"button"> &
  UseBulkDeleteControllerParams<RecordType, MutationOptionsError>;

/**
 * A button that deletes multiple selected records at once.
 *
 * Defaults to `mutationMode="undoable"`. When `mutationMode` is `pessimistic` or `optimistic`,
 * a confirmation dialog is shown before firing. Use within the `bulkActionsButtons` prop of
 * `<DataTable>` or inside `<BulkActionsToolbar>`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/bulk-delete-button BulkDeleteButton documentation}
 *
 * @example
 * import { BulkDeleteButton, BulkExportButton, DataTable, List } from '@/components/admin';
 *
 * export const PostList = () => (
 *   <List>
 *     <DataTable
 *       bulkActionsButtons={
 *         <>
 *           <BulkExportButton />
 *           <BulkDeleteButton />
 *         </>
 *       }
 *     >
 *       ...
 *     </DataTable>
 *   </List>
 * );
 */
function BulkDeleteButton<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
>(props: BulkDeleteButtonProps<RecordType, MutationOptionsError>) {
  const { mutationMode = "undoable" } = props;
  if (mutationMode === "undoable") {
    return (
      <BulkDeleteWithUndoButton
        {...(props as BulkDeleteButtonProps<RecordType, MutationOptionsError>)}
      />
    );
  }
  return (
    <BulkDeleteWithConfirmButton
      {...(props as BulkDeleteButtonProps<RecordType, MutationOptionsError>)}
    />
  );
}

/**
 * Bulk delete button variant that fires immediately and shows an undo notification.
 *
 * Equivalent to `<BulkDeleteButton mutationMode="undoable">`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/bulk-delete-button BulkDeleteButton documentation}
 */
function BulkDeleteWithUndoButton<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
>({
  icon = defaultIcon,
  label: labelProp,
  className,
  onClick,
  ref,
  ...props
}: BulkDeleteButtonProps<RecordType, MutationOptionsError>) {
  const { handleDelete, isPending } = useBulkDeleteController({
    ...props,
    mutationMode: "undoable",
  });
  const resource = useResourceContext(props);
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "delete",
    resource,
  });
  const getResourceLabel = useGetResourceLabel();
  const label = useResourceTranslation({
    resourceI18nKey: resource
      ? `resources.${resource}.action.delete`
      : undefined,
    baseI18nKey: "ra.action.delete",
    options: {
      name: resource ? getResourceLabel(resource, 1) : undefined,
    },
    userText: labelProp,
  });

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    handleDelete();
    onClick?.(e);
  };

  if (isAccessPending || !canAccess) return null;
  return (
    <Button
      ref={ref}
      variant="destructive"
      type="button"
      onClick={handleClick}
      disabled={isPending}
      aria-label={typeof label === "string" ? label : undefined}
      className={cn("h-9", className)}
    >
      {icon}
      {label}
    </Button>
  );
}

type BulkDeleteWithConfirmButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> = BulkDeleteButtonProps<RecordType, MutationOptionsError> & {
  confirmTitle?: ReactNode;
  confirmContent?: ReactNode;
  confirmColor?: "primary" | "warning";
};

/**
 * Bulk delete button variant that asks the user to confirm before firing the mutation.
 *
 * Equivalent to `<BulkDeleteButton mutationMode="pessimistic">` (or `optimistic`).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/bulk-delete-button BulkDeleteButton documentation}
 */
function BulkDeleteWithConfirmButton<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
>(props: BulkDeleteWithConfirmButtonProps<RecordType, MutationOptionsError>) {
  const {
    icon = defaultIcon,
    label: labelProp,
    className,
    onClick,
    ref,
    confirmTitle = "ra.message.bulk_delete_title",
    confirmContent = "ra.message.bulk_delete_content",
    confirmColor = "primary",
    mutationMode = "pessimistic",
    mutationOptions,
    ...rest
  } = props;
  const [isOpen, setOpen] = useState(false);
  const { selectedIds } = useListContext();
  const resource = useResourceContext(props);
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "delete",
    resource,
  });
  const translate = useTranslate();
  const getResourceLabel = useGetResourceLabel();

  const { handleDelete, isPending } = useBulkDeleteController({
    ...rest,
    mutationMode,
    mutationOptions: {
      ...mutationOptions,
      onSettled: (...args) => {
        if (mutationMode === "pessimistic") {
          setOpen(false);
        }
        mutationOptions?.onSettled?.(...args);
      },
    },
  });

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    setOpen(true);
  };

  const handleDialogClose = () => {
    setOpen(false);
  };

  const handleConfirm = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    if (mutationMode !== "pessimistic") {
      setOpen(false);
    }
    handleDelete();
    if (typeof onClick === "function") {
      onClick(e);
    }
  };

  const label = useResourceTranslation({
    resourceI18nKey: resource
      ? `resources.${resource}.action.delete`
      : undefined,
    baseI18nKey: "ra.action.delete",
    options: {
      name: resource ? getResourceLabel(resource, 1) : undefined,
    },
    userText: labelProp,
  });

  const nameOptions = {
    smart_count: selectedIds.length,
    name: translate(`resources.${resource}.forcedCaseName`, {
      smart_count: selectedIds.length,
      _: humanize(
        translate(`resources.${resource}.name`, {
          smart_count: selectedIds.length,
          _: resource ? inflect(resource, selectedIds.length) : undefined,
        }),
        true,
      ),
    }),
  };

  if (isAccessPending || !canAccess) return null;
  return (
    <Fragment>
      <Button
        ref={ref}
        variant="destructive"
        type="button"
        onClick={handleClick}
        disabled={isPending}
        aria-label={typeof label === "string" ? label : undefined}
        className={cn("h-9", className)}
      >
        {icon}
        {label}
      </Button>
      <Confirm
        isOpen={isOpen}
        loading={isPending}
        title={confirmTitle}
        content={confirmContent}
        confirmColor={confirmColor}
        titleTranslateOptions={nameOptions}
        contentTranslateOptions={nameOptions}
        onConfirm={handleConfirm}
        onClose={handleDialogClose}
      />
    </Fragment>
  );
}

const defaultIcon = <Trash />;

export {
  BulkDeleteButton,
  BulkDeleteWithUndoButton,
  BulkDeleteWithConfirmButton,
  type BulkDeleteButtonProps,
  type BulkDeleteWithConfirmButtonProps,
};
