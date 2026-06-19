import * as React from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { ExternalToast, ToasterProps } from "sonner";
import { Toaster, toast } from "sonner";
import { useTheme } from "@/hooks/use-theme";
import {
  CloseNotificationContext,
  type NotificationPayload,
  useAuthProvider,
  useAuthState,
  useNotificationContext,
  useTakeUndoableMutation,
  useTranslate,
} from "shadmin-core";

/**
 * Displays notifications triggered with the useNotify hook.
 *
 * Supports different notification types (info, success, warning, error) and undoable mutations.
 * Automatically adapts to the current theme (light/dark).
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/notification Notification documentation}
 * @see {@link https://marmelab.com/ra-core/usenotify/ useNotify hook}
 *
 * @example
 * // Trigger a notification
 * import { useNotify } from 'ra-core';
 *
 * const NotifyButton = () => {
 *   const notify = useNotify();
 *   const handleClick = () => {
 *     notify('Comment approved', { type: 'success' });
 *   };
 *   return <button onClick={handleClick}>Notify</button>;
 * };
 */
function Notification(props: ToasterProps) {
  const { notifications, takeNotification, resetNotifications } =
    useNotificationContext();
  const takeMutation = useTakeUndoableMutation();
  const [theme] = useTheme();
  const [currentNotification, setCurrentNotification] = useState<
    NotificationPayload | undefined
  >(undefined);
  const undoRef = useRef(false);

  // Capture translate via ref so the effect deps don't include it (avoids
  // re-firing the queue drain on locale change).
  const translate = useTranslate();
  const translateRef = useRef(translate);
  useEffect(() => {
    translateRef.current = translate;
  }, [translate]);

  const resetQueue = useCallback(() => {
    toast.dismiss();
    resetNotifications();
    setCurrentNotification(undefined);
    undoRef.current = false;
  }, [resetNotifications]);

  useEffect(() => {
    if (notifications.length === 0 || currentNotification) return;

    const notification = takeNotification();
    if (!notification) return;

    setCurrentNotification(notification);

    const { message, type = "info", notificationOptions } = notification;
    const { messageArgs, undoable, autoHideDuration } =
      notificationOptions || {};

    const beforeunload = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      const confirmationMessage = "";
      e.returnValue = confirmationMessage;
      return confirmationMessage;
    };

    let beforeunloadAttached = false;
    if (undoable) {
      window.addEventListener("beforeunload", beforeunload);
      beforeunloadAttached = true;
    }
    const detachBeforeunload = () => {
      if (beforeunloadAttached) {
        window.removeEventListener("beforeunload", beforeunload);
        beforeunloadAttached = false;
      }
    };

    // Only consume a mutation for undoable notifications, and only once.
    // Captured here so handleUndo and handleExited share the same instance.
    const mutation = undoable ? takeMutation() : undefined;

    const handleExited = () => {
      // If the user clicked Undo, handleUndo already fired the mutation
      // with isUndo: true. Skip firing it again here to avoid a double-fire.
      if (!undoRef.current && undoable && mutation) {
        mutation({ isUndo: false });
      }
      undoRef.current = false;
      detachBeforeunload();
      setCurrentNotification(undefined);
    };

    const handleUndo = () => {
      undoRef.current = true;
      if (mutation) {
        mutation({ isUndo: true });
      }
      detachBeforeunload();
      // Sonner doesn't fire onDismiss when the action button is clicked, so
      // release the queue slot here. handleExited won't fire for this toast.
      setCurrentNotification(undefined);
    };

    const localTranslate = translateRef.current;
    const finalMessage = message
      ? typeof message === "string"
        ? localTranslate(message, messageArgs)
        : React.isValidElement(message)
          ? message
          : undefined
      : undefined;

    // Map ra-core's autoHideDuration to sonner's duration:
    //   undefined -> omit (use Toaster's default)
    //   null      -> Infinity (persistent toast)
    //   number    -> use as-is
    const toastOptions: ExternalToast = {
      action: undoable
        ? {
            label: localTranslate("ra.action.undo"),
            onClick: handleUndo,
          }
        : undefined,
      onDismiss: handleExited,
      onAutoClose: handleExited,
    };
    if (autoHideDuration === null) {
      toastOptions.duration = Infinity;
    } else if (autoHideDuration !== undefined) {
      toastOptions.duration = autoHideDuration;
    }

    // Sonner exposes aria-live at the container level only, so we annotate
    // error/warning toasts with a className so assistive tech / custom CSS
    // can target them, and the audit trail keeps a stable per-type hook.
    if (type === "error" || type === "warning") {
      toastOptions.className = `notification-${type}`;
    }

    toast[type](finalMessage, toastOptions);

    return () => {
      if (beforeunloadAttached) {
        window.removeEventListener("beforeunload", beforeunload);
        beforeunloadAttached = false;
      }
    };
  }, [
    notifications.length,
    currentNotification,
    takeMutation,
    takeNotification,
  ]);

  const handleRequestClose = useCallback(() => {
    // Dismiss all toasts
    toast.dismiss();
  }, []);

  // Only watch auth state when an authProvider is configured. Otherwise the
  // dependent hooks (useLogout, useQuery, useNavigate) would throw in unit
  // tests / storybook that mount Notification without the Admin context.
  const authProvider = useAuthProvider();

  const defaultToasterProps: Partial<ToasterProps> = {
    richColors: true,
    closeButton: true,
    position: "bottom-center",
  };

  return (
    <CloseNotificationContext.Provider value={handleRequestClose}>
      {authProvider != null ? (
        <LogoutQueueReset resetQueue={resetQueue} />
      ) : null}
      <Toaster {...defaultToasterProps} theme={theme} {...props} />
    </CloseNotificationContext.Provider>
  );
}

/**
 * Listens for the user becoming unauthenticated (e.g. after logout) and
 * clears the notification queue so a stale notification doesn't surface
 * against the login screen or a subsequent session.
 */
function LogoutQueueReset({ resetQueue }: { resetQueue: () => void }) {
  const { authenticated } = useAuthState(undefined, false);
  const wasAuthenticatedRef = useRef<boolean | undefined>(undefined);
  useEffect(() => {
    if (wasAuthenticatedRef.current === true && authenticated === false) {
      resetQueue();
    }
    wasAuthenticatedRef.current = authenticated;
  }, [authenticated, resetQueue]);
  return null;
}

export { Notification };
