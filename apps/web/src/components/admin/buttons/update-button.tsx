import * as React from "react";
import { Fragment, useState, type Ref } from "react";
import { humanize, singularize } from "inflection";
import { RefreshCw } from "lucide-react";
import {
  type RaRecord,
  type UseUpdateControllerParams,
  useCanAccess,
  useGetRecordRepresentation,
  useRecordContext,
  useResourceContext,
  useResourceTranslation,
  useTranslate,
  useUpdateController,
} from "shadmin-core";

import { Button } from "@/components/ui/button";
import { Confirm } from "@/components/admin/feedback/confirm";
import type { UnknownValue } from "@/lib/unknown-types";

interface UpdateWithUndoButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> extends Omit<
    UseUpdateControllerParams<RecordType, MutationOptionsError>,
    "mutationMode"
  > {
  className?: string;
  data: Partial<RecordType>;
  icon?: React.ReactNode;
  label?: string;
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
  ref?: Ref<HTMLButtonElement>;
  variant?: React.ComponentProps<typeof Button>["variant"];
}

interface UpdateWithConfirmButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> extends UseUpdateControllerParams<RecordType, MutationOptionsError> {
  className?: string;
  confirmContent?: React.ReactNode;
  confirmTitle?: React.ReactNode;
  data: Partial<RecordType>;
  icon?: React.ReactNode;
  label?: string;
  onClick?: React.MouseEventHandler<HTMLButtonElement>;
  ref?: Ref<HTMLButtonElement>;
  titleTranslateOptions?: object;
  contentTranslateOptions?: object;
  variant?: React.ComponentProps<typeof Button>["variant"];
}

type UpdateButtonProps<
  RecordType extends RaRecord = RaRecord,
  MutationOptionsError = UnknownValue,
> =
  | ({ mutationMode?: "undoable" } & UpdateWithUndoButtonProps<
      RecordType,
      MutationOptionsError
    >)
  | ({
      mutationMode: "pessimistic" | "optimistic";
    } & UpdateWithConfirmButtonProps<RecordType, MutationOptionsError>);

/**
 * Updates the current record with a fixed data payload.
 *
 * Defaults to `mutationMode="undoable"`. When `mutationMode` is `pessimistic` or `optimistic`,
 * it shows a confirmation dialog before firing the mutation. Must be used inside a
 * `RecordContext` and `ResourceContext`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/update-button UpdateButton documentation}
 *
 * @example
 * import { UpdateButton } from '@/components/admin';
 *
 * const ResetViewsButton = () => (
 *   <UpdateButton label="Reset Views" data={{ views: 0 }} />
 * );
 */
function UpdateButton(props: UpdateButtonProps) {
  const { mutationMode = "undoable", ...rest } = props;
  return mutationMode === "undoable" ? (
    <UpdateWithUndoButton {...(rest as UpdateWithUndoButtonProps)} />
  ) : (
    <UpdateWithConfirmButton
      mutationMode={mutationMode}
      {...(rest as UpdateWithConfirmButtonProps)}
    />
  );
}

/**
 * Update button variant that asks the user to confirm before firing the mutation.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/update-button UpdateButton documentation}
 */
function UpdateWithConfirmButton(props: UpdateWithConfirmButtonProps) {
  const {
    className,
    confirmTitle: confirmTitleProp,
    confirmContent: confirmContentProp,
    data,
    icon = defaultIcon,
    label: labelProp,
    mutationMode = "pessimistic",
    onClick,
    ref,
    titleTranslateOptions = emptyObject,
    contentTranslateOptions = emptyObject,
    variant = "outline",
    ...rest
  } = props;
  const translate = useTranslate();
  const resource = useResourceContext(props);
  const record = useRecordContext(props);
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "edit",
    resource,
    record,
  });
  const [isOpen, setOpen] = useState(false);

  const { handleUpdate, isPending } = useUpdateController({
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
    resourceI18nKey: `resources.${resource}.action.update`,
    baseI18nKey: "ra.action.update",
    options: {
      name: resourceName,
      recordRepresentation,
    },
    userText: labelProp,
  });
  const confirmTitle = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.message.bulk_update_title`,
    baseI18nKey: "ra.message.bulk_update_title",
    options: {
      recordRepresentation,
      name: resourceName,
      id: record?.id,
      smart_count: 1,
      ...titleTranslateOptions,
    },
    userText: confirmTitleProp,
  });
  const confirmContent = useResourceTranslation({
    resourceI18nKey: `resources.${resource}.message.bulk_update_content`,
    baseI18nKey: "ra.message.bulk_update_content",
    options: {
      recordRepresentation,
      name: resourceName,
      id: record?.id,
      smart_count: 1,
      ...contentTranslateOptions,
    },
    userText: confirmContentProp,
  });

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
        onConfirm={handleConfirm}
        onClose={handleDialogClose}
      />
    </Fragment>
  );
}

/**
 * Update button variant that fires immediately with undo capability.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/update-button UpdateButton documentation}
 */
function UpdateWithUndoButton(props: UpdateWithUndoButtonProps) {
  const {
    className,
    data,
    icon = defaultIcon,
    label: labelProp,
    onClick,
    ref,
    variant = "outline",
    ...rest
  } = props;
  const record = useRecordContext(props);
  const resource = useResourceContext(props);
  const { canAccess, isPending: isAccessPending } = useCanAccess({
    action: "edit",
    resource,
    record,
  });
  const translate = useTranslate();
  const { handleUpdate, isPending } = useUpdateController({
    ...rest,
    mutationMode: "undoable",
  });

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
    resourceI18nKey: `resources.${resource}.action.update`,
    baseI18nKey: "ra.action.update",
    options: {
      name: resourceName,
      recordRepresentation,
    },
    userText: labelProp,
  });

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (!record) {
      throw new Error(
        "The UpdateWithUndoButton must be used inside a RecordContext.Provider or must be passed a record prop.",
      );
    }
    handleUpdate(data);
    if (typeof onClick === "function") {
      onClick(e);
    }
    e.stopPropagation();
  };

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
const emptyObject = {};

export {
  UpdateButton,
  type UpdateButtonProps,
  UpdateWithConfirmButton,
  type UpdateWithConfirmButtonProps,
  UpdateWithUndoButton,
  type UpdateWithUndoButtonProps,
};
