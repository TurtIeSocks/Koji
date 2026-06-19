import { AlertCircle, CheckCircle } from "lucide-react";
import { useTranslate } from "shadmin-core";
import * as React from "react";
import { type ComponentType, type MouseEventHandler, useCallback } from "react";
import { DialogPrimitive } from "@/components/ui/dialog";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

/**
 * Generic confirmation dialog component for destructive actions.
 *
 * Displays a dialog with customizable title, content, and action buttons.
 * Used internally for delete operations and other confirmations.
 * Supports custom icons, button labels, and loading states.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/confirm Confirm documentation}
 *
 * @example
 * import { useState } from "react";
 * import { useDelete, useRecordContext, useResourceContext, useRedirect } from "shadmin-core";
 * import { Button } from "@/components/ui/button";
 * import { Confirm } from "@/components/admin/feedback/confirm";
 *
 * const DeleteButton = () => {
 *   const resource = useResourceContext();
 *   const record = useRecordContext();
 *   const [isOpen, setIsOpen] = useState(false);
 *   const [deleteOne, { isPending }] = useDelete();
 *   const redirect = useRedirect();
 *
 *   const handleDelete = () => {
 *     deleteOne(
 *       resource,
 *       { id: record?.id, previousData: record },
 *       {
 *         onSuccess: () => {
 *           setIsOpen(false);
 *           redirect("list", resource);
 *         },
 *       },
 *     );
 *   };
 *
 *   return (
 *     <>
 *       <Button variant="destructive" onClick={() => setIsOpen(true)}>
 *         Delete
 *       </Button>
 *       <Confirm
 *         isOpen={isOpen}
 *         title="Are you sure you want to delete this element?"
 *         content="This action cannot be undone."
 *         onConfirm={handleDelete}
 *         onClose={() => setIsOpen(false)}
 *         loading={isPending}
 *       />
 *     </>
 *   );
 * };
 */
function Confirm(props: ConfirmProps) {
  const {
    className,
    isOpen = false,
    loading,
    title,
    content,
    cancel = "ra.action.cancel",
    confirm = "ra.action.confirm",
    confirmColor = "primary",
    ConfirmIcon = CheckCircle,
    CancelIcon = AlertCircle,
    onClose,
    onConfirm,
    translateOptions = {},
    titleTranslateOptions = translateOptions,
    contentTranslateOptions = translateOptions,
    // Dialog (Radix Root) pass-through props
    modal,
    defaultOpen,
    ...rest
  } = props;

  const translate = useTranslate();

  const handleClose = useCallback(
    (e?: React.MouseEvent) => {
      onClose(e);
    },
    [onClose],
  );

  const handleConfirm = useCallback(
    (e: React.MouseEvent<HTMLButtonElement>) => {
      e.stopPropagation();
      onConfirm(e);
    },
    [onConfirm],
  );

  const handleClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();
  }, []);

  return (
    <Dialog
      open={isOpen}
      onOpenChange={() => handleClose()}
      modal={modal}
      defaultOpen={defaultOpen}
    >
      <DialogContent className={className} onClick={handleClick} {...rest}>
        <DialogHeader>
          <DialogTitle>
            {typeof title === "string"
              ? translate(title, { _: title, ...titleTranslateOptions })
              : title}
          </DialogTitle>
          {typeof content === "string" ? (
            <DialogDescription>
              {translate(content, {
                _: content,
                ...contentTranslateOptions,
              })}
            </DialogDescription>
          ) : (
            content
          )}
        </DialogHeader>
        <DialogFooter>
          <Button
            variant="ghost"
            disabled={loading}
            onClick={handleClose}
            className="gap-1"
          >
            <CancelIcon className="size-5" />
            {translate(cancel, { _: cancel })}
          </Button>
          <Button
            autoFocus
            disabled={loading}
            onClick={handleConfirm}
            className="gap-1"
            variant={confirmColor === "warning" ? "destructive" : "default"}
          >
            <ConfirmIcon className="size-5" />
            {translate(confirm, { _: confirm })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/**
 * Props for `<Confirm>`.
 *
 * Extends `React.ComponentProps<typeof DialogPrimitive.Content>` so any
 * HTML attribute (e.g. `aria-*`, `data-*`) or Radix `DialogContent` prop
 * is forwarded to the underlying `<DialogContent>` element.
 * Radix `Dialog.Root`-level props (`modal`, `defaultOpen`, `onOpenChange`)
 * are also accepted and forwarded to the root `<Dialog>`.
 */
interface ConfirmProps
  extends Omit<
    React.ComponentProps<typeof DialogPrimitive.Content>,
    "title" | "content" | "onClose"
  > {
  cancel?: string;
  className?: string;
  confirm?: string;
  confirmColor?: "primary" | "warning";
  ConfirmIcon?: ComponentType;
  CancelIcon?: ComponentType;
  content?: React.ReactNode;
  isOpen?: boolean;
  loading?: boolean;
  onClose: (event?: React.MouseEvent) => void;
  onConfirm: MouseEventHandler;
  title: React.ReactNode;
  /**
   * @deprecated use `titleTranslateOptions` and `contentTranslateOptions` instead
   */
  translateOptions?: object;
  titleTranslateOptions?: object;
  contentTranslateOptions?: object;
  /** Forwarded to Radix `Dialog.Root` — sets modality. */
  modal?: React.ComponentProps<typeof DialogPrimitive.Root>["modal"];
  /** Forwarded to Radix `Dialog.Root` — uncontrolled initial open state. */
  defaultOpen?: React.ComponentProps<
    typeof DialogPrimitive.Root
  >["defaultOpen"];
}

export { Confirm, type ConfirmProps };
