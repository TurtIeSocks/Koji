import type { FallbackProps } from "react-error-boundary";
import { useResetErrorBoundaryOnLocationChange, Translate } from "shadmin-core";
import { CircleAlert, History } from "lucide-react";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import type {
  HtmlHTMLAttributes,
  ErrorInfo,
  ComponentType,
  ReactNode,
} from "react";
import { Title } from "@/components/admin/layout/title";

/**
 * App-wide error component for displaying error boundaries.
 *
 * Displays the error message and a back button.
 * In development mode, also shows the component stack trace in an expandable accordion.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/error Error documentation}
 */
function Error(props: InternalErrorProps) {
  const {
    error,
    errorInfo,
    resetErrorBoundary,
    errorComponent,
    title,
    ...rest
  } = props;

  useResetErrorBoundaryOnLocationChange(resetErrorBoundary);

  if (errorComponent) {
    const ErrorComponent = errorComponent;
    return (
      <ErrorComponent
        error={error}
        errorInfo={errorInfo}
        resetErrorBoundary={resetErrorBoundary}
      />
    );
  }

  const errorMessage: string =
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
      ? (error.message ?? "")
      : typeof error === "string"
        ? error
        : String(error ?? "Unknown error");

  return (
    <div className="flex flex-col items-center md:p-16 gap-5" {...rest}>
      {title ? (
        <Title defaultTitle={typeof title === "string" ? title : undefined} />
      ) : null}
      <h1 className="flex items-center text-3xl mt-5 mb-5 gap-3" role="alert">
        <CircleAlert className="w-[2em] h-[2em]" />
        <Translate i18nKey="ra.page.error" />
      </h1>
      <div>
        <Translate i18nKey="ra.message.error" />
      </div>
      {import.meta.env.DEV && (
        <>
          <Accordion
            type="multiple"
            className="mt-1 p-2 bg-secondary w-full lg:w-150"
          >
            <AccordionItem value="error">
              <AccordionTrigger className="py-2">
                <Translate i18nKey={errorMessage}>{errorMessage}</Translate>
              </AccordionTrigger>
              <AccordionContent className="whitespace-pre-wrap pt-1">
                <pre className="text-xs">{errorInfo?.componentStack}</pre>
              </AccordionContent>
            </AccordionItem>
          </Accordion>

          <p className="text-center ">
            Need help with this error? Try the following:
          </p>
          <div>
            <ul className="list-disc">
              <li>
                Check the{" "}
                <a
                  className="text-primary underline-offset-4 hover:underline"
                  href="https://shadmin.turtlesocks.dev/docs"
                >
                  shadmin documentation
                </a>
              </li>
              <li>
                Search on{" "}
                <a
                  className="text-primary underline-offset-4 hover:underline"
                  href="https://stackoverflow.com/questions/tagged/shadmin"
                >
                  StackOverflow
                </a>{" "}
                for community answers
              </li>
              <li>
                Get help from the core team via{" "}
                <a
                  className="text-primary underline-offset-4 hover:underline"
                  href="https://shadmin.turtlesocks.dev/"
                >
                  Shadcn Enterprise Edition
                </a>
              </li>
            </ul>
          </div>
        </>
      )}
      <div className="mt-8">
        <Button onClick={goBack}>
          <History />
          <Translate i18nKey="ra.action.back" />
        </Button>
      </div>
    </div>
  );
}

interface InternalErrorProps
  extends Omit<HtmlHTMLAttributes<HTMLDivElement>, "title">,
    FallbackProps {
  className?: string;
  errorInfo?: ErrorInfo;
  errorComponent?: ComponentType<ErrorProps>;
  /**
   * When provided, injects a `<Title>` portal to update the app-bar page title.
   * Pass `false` to explicitly suppress a title. Matches MUI react-admin `Error` behaviour.
   */
  title?: ReactNode | string | false;
}

interface ErrorProps
  extends Pick<FallbackProps, "error" | "resetErrorBoundary"> {
  errorInfo?: ErrorInfo;
  /**
   * When provided, injects a `<Title>` portal to update the app-bar page title.
   * Pass `false` to explicitly suppress a title.
   */
  title?: ReactNode | string | false;
}

function goBack() {
  window.history.go(-1);
}

export { Error, type ErrorProps };
