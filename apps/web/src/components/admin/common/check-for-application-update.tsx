import { useEffect, useRef, useState, type ReactNode } from "react";

import { ApplicationUpdatedNotification } from "@/components/admin/common/application-updated-notification";

const ONE_HOUR = 3600000;

interface CheckForApplicationUpdateProps {
  /**
   * Interval in milliseconds between two checks. Defaults to 3600000 (1 hour).
   */
  interval?: number;
  /**
   * URL to download to check for code update. Defaults to
   * `window.location.origin + "/index.html"`.
   */
  url?: string;
  /**
   * Notification rendered when a new version is detected.
   * Defaults to `<ApplicationUpdatedNotification />`.
   */
  notification?: ReactNode;
  /**
   * When true, the polling is skipped.
   */
  disabled?: boolean;
  /**
   * Options forwarded to the internal `fetch()` call — allows custom headers,
   * credentials, abort signal, etc. The `cache` option is always overridden to
   * `"no-store"` so change detection remains reliable.
   *
   * @see https://developer.mozilla.org/en-US/docs/Web/API/fetch#options
   */
  fetchOptions?: RequestInit;
  /**
   * Called when a new version is detected (hash change), in addition to
   * rendering the `notification`. Useful for side effects such as analytics or
   * auto-reloading after a grace period.
   */
  onNewVersionAvailable?: () => void;
}

/**
 * Polls a URL at a regular interval, hashes the response body, and renders a
 * notification when the hash changes — useful for detecting a new deployment
 * of the application while the user has the page open.
 *
 * Pair it with [`<ApplicationUpdatedNotification>`](./ApplicationUpdatedNotification.md)
 * (the default) to offer a "reload" action when an update is available.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/check-for-application-update CheckForApplicationUpdate documentation}
 *
 * @example
 * import { CheckForApplicationUpdate } from "@/components/admin/common/check-for-application-update";
 *
 * const MyLayout = ({ children }) => (
 *   <Layout>
 *     {children}
 *     <CheckForApplicationUpdate />
 *   </Layout>
 * );
 */
function CheckForApplicationUpdate(props: CheckForApplicationUpdateProps) {
  const {
    interval = ONE_HOUR,
    url,
    notification = <ApplicationUpdatedNotification />,
    disabled = import.meta.env.DEV,
    fetchOptions,
    onNewVersionAvailable,
  } = props;

  const [hasNewVersion, setHasNewVersion] = useState(false);
  const initialHashRef = useRef<number | null>(null);

  useEffect(() => {
    if (disabled) {
      return;
    }

    const targetUrl = url ?? `${window.location.origin}/index.html`;
    let cancelled = false;

    const fetchAndHash = async (): Promise<number | null> => {
      try {
        const response = await fetch(targetUrl, {
          ...fetchOptions,
          cache: "no-store",
        });
        if (!response.ok) {
          return null;
        }
        const body = await response.text();
        return hashString(body);
      } catch {
        return null;
      }
    };

    const check = async () => {
      const nextHash = await fetchAndHash();
      if (cancelled || nextHash === null) {
        return;
      }
      if (initialHashRef.current === null) {
        initialHashRef.current = nextHash;
        return;
      }
      if (nextHash !== initialHashRef.current) {
        setHasNewVersion(true);
        onNewVersionAvailable?.();
      }
    };

    // Initial fetch to record the baseline hash, then poll on the interval.
    void check();
    const timer = window.setInterval(() => {
      void check();
    }, interval);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [disabled, fetchOptions, interval, onNewVersionAvailable, url]);

  if (!hasNewVersion) {
    return null;
  }

  return <>{notification}</>;
}

/**
 * Tiny, synchronous, non-cryptographic string hash (djb2).
 * Crypto.subtle would be async and is overkill for change detection.
 */
function hashString(input: string): number {
  let hash = 5381;
  for (let i = 0; i < input.length; i += 1) {
    hash = (hash * 33) ^ input.charCodeAt(i);
  }
  // Force unsigned 32-bit integer so the value is stable for comparison.
  return hash >>> 0;
}

export { CheckForApplicationUpdate, type CheckForApplicationUpdateProps };
