import type { useNotify } from "shadmin-core";

type NotifyFn = ReturnType<typeof useNotify>;

const getErrorMessage = (error: unknown): string | undefined => {
  if (typeof error === "string") return error;
  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof (error as { message: unknown }).message === "string" &&
    (error as { message: string }).message
  ) {
    return (error as { message: string }).message;
  }
  return undefined;
};

/**
 * Surface an authentication error via `useNotify()`. Falls back to the
 * translated `ra.auth.sign_in_error` key when the error has no usable
 * message.
 */
export const notifyAuthError = (notify: NotifyFn, error: unknown) => {
  const message = getErrorMessage(error);
  notify(message ?? "ra.auth.sign_in_error", {
    type: "error",
    messageArgs: { _: message },
  });
};
