import { useEffect, useState, type ReactNode } from "react";
import { WifiOff } from "lucide-react";
import { useTranslate } from "shadmin-core";

import { cn } from "@/lib/utils";

interface OfflineProps {
  children?: ReactNode;
  className?: string;
}

/**
 * Renders its children only when the user is offline.
 *
 * Tracks `navigator.onLine` and listens for the `online` / `offline` events
 * on `window`. When no children are provided, a default banner is rendered
 * at the top of the viewport announcing that the user is offline.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/offline Offline documentation}
 *
 * @example
 * import { Offline } from "@/components/admin/feedback/offline";
 *
 * const App = () => (
 *   <>
 *     <Offline />
 *     <Routes />
 *   </>
 * );
 *
 * @example <caption>Custom content</caption>
 * <Offline>
 *   <p>Working from cache — changes will sync once you reconnect.</p>
 * </Offline>
 */
function Offline(props: OfflineProps) {
  const { children, className } = props;
  const translate = useTranslate();

  const [isOffline, setIsOffline] = useState<boolean>(() =>
    typeof navigator !== "undefined" ? !navigator.onLine : false,
  );

  useEffect(() => {
    const handleOnline = () => setIsOffline(false);
    const handleOffline = () => setIsOffline(true);

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  if (!isOffline) {
    return null;
  }

  if (children !== undefined) {
    return <div className={cn(className)}>{children}</div>;
  }

  return (
    <div
      role="status"
      aria-live="polite"
      className={cn(
        "fixed top-0 inset-x-0 z-50 bg-destructive text-destructive-foreground px-4 py-2 text-sm text-center flex items-center justify-center gap-2",
        className,
      )}
    >
      <WifiOff className="size-4 shrink-0" aria-hidden="true" />
      <span>
        {translate("ra.notification.offline", { _: "You are offline" })}
      </span>
    </div>
  );
}

export { Offline, type OfflineProps };
