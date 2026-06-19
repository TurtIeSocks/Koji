import { Sparkles } from "lucide-react";
import { useTranslate } from "shadmin-core";
import type { HTMLAttributes } from "react";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";

interface ApplicationUpdatedNotificationProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "children"> {
  className?: string;
  /**
   * Translation key (or literal string) for the notification body.
   * Also accepted as `notificationText` (MUI alias).
   */
  message?: string;
  /** @alias message — MUI-compatible name for the notification text. */
  notificationText?: string;
  /**
   * Translation key (or literal string) for the reload button label.
   * Also accepted as `updateText` (MUI alias).
   */
  buttonLabel?: string;
  /** @alias buttonLabel — MUI-compatible name for the button label. */
  updateText?: string;
  onReload?: () => void;
}

/**
 * A floating banner informing the user that a new version of the application
 * is available, with a button to reload the page.
 *
 * Designed to be rendered as the default notification of
 * [`<CheckForApplicationUpdate>`](./CheckForApplicationUpdate.md), but it can
 * also be used standalone to advertise an available update.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/application-updated-notification ApplicationUpdatedNotification documentation}
 *
 * @example
 * import { ApplicationUpdatedNotification } from "@/components/admin/common/application-updated-notification";
 *
 * const Notice = () => <ApplicationUpdatedNotification />;
 */
function ApplicationUpdatedNotification(
  props: ApplicationUpdatedNotificationProps,
) {
  const {
    className,
    message,
    notificationText,
    buttonLabel,
    updateText,
    onReload,
    ...rest
  } = props;

  const finalMessage =
    message ??
    notificationText ??
    "ra.notification.application_update_available";
  const finalButtonLabel = buttonLabel ?? updateText ?? "ra.action.reload";

  const translate = useTranslate();

  const handleReload = () => {
    if (onReload) {
      onReload();
      return;
    }
    if (typeof window !== "undefined") {
      window.location.reload();
    }
  };

  return (
    <Card
      role="alert"
      aria-live="polite"
      className={cn(
        "fixed bottom-4 left-1/2 -translate-x-1/2 z-50 max-w-md w-[calc(100%-2rem)] flex-row items-center gap-3 px-4 py-3",
        className,
      )}
      {...rest}
    >
      <Sparkles className="size-5 text-primary shrink-0" aria-hidden="true" />
      <p className="text-sm flex-1">
        {translate(finalMessage, { _: finalMessage })}
      </p>
      <Button size="sm" onClick={handleReload}>
        {translate(finalButtonLabel, { _: finalButtonLabel })}
      </Button>
    </Card>
  );
}

export {
  ApplicationUpdatedNotification,
  type ApplicationUpdatedNotificationProps,
};
