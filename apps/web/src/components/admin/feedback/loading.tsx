import { Translate, useTimeout } from "shadmin-core";
import { Spinner } from "@/components/admin/feedback/spinner";

/**
 * Loading indicator used for slow element or page loads.
 *
 * Displays a spinner and customizable loading messages.
 * Automatically shown by the default Layout when page loading takes more than 1 second.
 * Works as a fallback for React Suspense boundaries.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/loading Loading documentation}
 */
function Loading(props: LoadingProps) {
  const {
    loadingPrimary = "ra.page.loading",
    loadingSecondary = "ra.message.loading",
    delay = 1000,
    ...rest
  } = props;
  const oneSecondHasPassed = useTimeout(delay);
  return oneSecondHasPassed ? (
    <div
      className="flex flex-col justify-center items-center h-full"
      role="status"
      aria-live="polite"
      {...rest}
    >
      <div className="text-center font-sans text-muted-foreground py-1">
        <Spinner size="large" className="size-9" />
        <h5 className="mt-3 text-2xl text-secondary-foreground">
          <Translate i18nKey={loadingPrimary}>{loadingPrimary}</Translate>
        </h5>
        <p className="text-primary">
          <Translate i18nKey={loadingSecondary}>{loadingSecondary}</Translate>
        </p>
      </div>
    </div>
  ) : null;
}

interface LoadingProps {
  loadingPrimary?: string;
  loadingSecondary?: string;
  delay?: number;
}

type LoadingPageProps = LoadingProps;

export {
  Loading,
  Loading as LoadingPage,
  type LoadingProps,
  type LoadingPageProps,
};
