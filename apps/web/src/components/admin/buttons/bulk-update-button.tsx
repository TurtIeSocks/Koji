import * as React from "react";
import { Fragment, useState, type Ref } from "react";
import { humanize, inflect } from "inflection";
import { RefreshCw } from "lucide-react";
import {
  type MutationMode,
  type RaRecord,
  type UseBulkUpdateControllerParams,
  useCanAccess,
  useBulkUpdateController,
  useListContext,
  useResourceContext,
  useResourceTranslation,
  useTranslate,
} from "shadmin-core";

import { Button } from "@/components/ui/button";
import { Confirm } from "@/components/admin/feedback/confirm";
import type { UnknownValue } from "@/lib/unknown-types";

interface BulkUpdateWithUndoButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> extends Omit<
    UseBulkUpdateControllerParams<RecordType, MutationOptionsError>,
    "mutationMode"
  > {
  className?: string;
  data?: Partial<RecordType>;
  icon?: React.ReactNode;
  label?: string;
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
  ref?: Ref<HTMLButtonElement>;
  variant?: React.ComponentProps<typeof Button>["variant"];
}

interface BulkUpdateWithConfirmButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> extends UseBulkUpdateControllerParams<RecordType, MutationOptionsError> {
  className?: string;
  confirmContent?: React.ReactNode;
  confirmTitle?: React.ReactNode;
  data?: Partial<RecordType>;
  icon?: React.ReactNode;
  label?: string;
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
  ref?: Ref<HTMLButtonElement>;
  variant?: React.ComponentProps<typeof Button>["variant"];
}

type BulkUpdateButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> = { mutationMode?: MutationMode } & (
  | BulkUpdateWithUndoButtonProps<RecordType, MutationOptionsError>
  | BulkUpdateWithConfirmButtonProps<RecordType, MutationOptionsError>
);

/**
 * Updates the selected records with a fixed data payload.
 *
 * Defaults to `mutationMode="undoable"`. When `mutationMode` is `pessimistic` or `optimistic`,
 * a confirmation dialog is shown before the mutation fires. Must be used inside a
 * `ListContext` (e.g., inside a `<DataTable bulkActionsButtons>`).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/bulk-update-button BulkUpdateButton documentation}
 *
 * @example
 * import { BulkUpdateButton, DataTable, List } from '@/components/admin';
 *
 * const ResetViewsButton = () => (
 *   <BulkUpdateButton label="Reset Views" data={{ views: 0 }} />
 * );
 *
 * export const PostList = () => (
 *   <List>
 *     <DataTable bulkActionsButtons={<ResetViewsButton />}>
 *       ...
 *     </DataTable>
 *   </List>
 * );
 */
function BulkUpdateButton(props: BulkUpdateButtonProps) {
  const { mutationMode = "undoable", ...rest } = props;
  return mutationMode === "undoable" ? (
    <BulkUpdateWithUndoButton {...(rest as BulkUpdateWithUndoButtonProps)} />
  ) : (
    <BulkUpdateWithConfirmButton
      mutationMode={mutationMode}
      {...(rest as BulkUpdateWithConfirmButtonProps)}
    />
  );
}

/**
 * Bulk update button variant that asks the user to confirm before firing the mutation.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/bulk-update-button BulkUpdateButton documentation}
 */
function BulkUpdateWithConfirmButton(props: BulkUpdateWithConfirmButtonProps) {
  const {
    className,
    confirmTitle = "ra.message.bulk_update_title",
    confirmContent = "ra.message.bulk_update_content",
    data = defaultData,
    icon = defaultIcon,
    label: labelProp,
    mutationMode = "pessimistic",
    onClick,
    ref,
    variant = "outline",
    ...rest
  } = props;
  const translate = useTranslate();
  const resource = useResourceContext(props);
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "edit",
    resource,
  });
  const [isOpen, setOpen] = useState(false);
  const { selectedIds } = useListContext();

  const { handleUpdate, isPending } = useBulkUpdateController({
    ...rest,
    mutationMode,
    mutationOptions: {
      ...rest.mutationOptions,
      onSettled: (...args) => {
        if (mutationMode === "pessimistic") {
          setOpen(false);
        }
        rest.mutationOptions?.onSettled?.(...args);
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
    handleUpdate(data);
    if (typeof onClick === "function") {
      onClick(e);
    }
  };

  const label = useResourceTranslation({
    resourceI18nKey: resource
      ? `resources.${resource}.action.update`
      : undefined,
    baseI18nKey: "ra.action.update",
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
        type="button"
        variant={variant}
        onClick={handleClick}
        disabled={isPending}
        aria-label={typeof label === "string" ? label : undefined}
        className={className}
      >
        {icon}
        {label}
      </Button>
      <Confirm
        isOpen={isOpen}
        loading={isPending}
        title={confirmTitle}
        content={confirmContent}
        titleTranslateOptions={nameOptions}
        contentTranslateOptions={nameOptions}
        onConfirm={handleConfirm}
        onClose={handleDialogClose}
      />
    </Fragment>
  );
}

/**
 * Bulk update button variant that fires immediately with undo capability.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/bulk-update-button BulkUpdateButton documentation}
 */
function BulkUpdateWithUndoButton(props: BulkUpdateWithUndoButtonProps) {
  const {
    className,
    data = defaultData,
    icon = defaultIcon,
    label: labelProp,
    onClick,
    ref,
    variant = "outline",
    ...rest
  } = props;
  const resource = useResourceContext(props);
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "edit",
    resource,
  });
  const { handleUpdate, isPending } = useBulkUpdateController({
    ...rest,
    mutationMode: "undoable",
  });

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    handleUpdate(data);
    if (typeof onClick === "function") {
      onClick(e);
    }
  };

  const label = useResourceTranslation({
    resourceI18nKey: resource
      ? `resources.${resource}.action.update`
      : undefined,
    baseI18nKey: "ra.action.update",
    userText: labelProp,
  });

  if (isAccessPending || !canAccess) return null;
  return (
    <Button
      ref={ref}
      type="button"
      variant={variant}
      onClick={handleClick}
      disabled={isPending}
      aria-label={typeof label === "string" ? label : undefined}
      className={className}
    >
      {icon}
      {label}
    </Button>
  );
}

const defaultIcon = <RefreshCw />;
const defaultData = {};

export {
  BulkUpdateButton,
  BulkUpdateWithConfirmButton,
  BulkUpdateWithUndoButton,
  type BulkUpdateButtonProps,
  type BulkUpdateWithConfirmButtonProps,
  type BulkUpdateWithUndoButtonProps,
};
