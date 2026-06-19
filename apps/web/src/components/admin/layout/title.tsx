import { useEffect, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";
import { useTranslate } from "shadmin-core";
import { TITLE_PORTAL_ID } from "@/lib/title-portal-id";
import { cn } from "@/lib/utils";

type TitleProps = {
  /**
   * The title to display. If a string, it is translated using the i18n provider.
   * Pass a React element to render a custom title.
   */
  title?: string | ReactElement;
  /**
   * Fallback title used when `title` is undefined.
   * Translated if a string.
   */
  defaultTitle?: string | ReactElement;
  /**
   * Optional CSS class applied to the rendered title element.
   */
  className?: string;
};

/**
 * Renders a page title inside the `<TitlePortal>` slot in the app bar.
 *
 * `<Title>` teleports its content into the element with id
 * `ra-title-portal` (rendered by `<TitlePortal>` inside `<AppBar>`). This
 * mirrors the upstream react-admin `<Title>` API so views can declare their
 * own page title without knowing where it lives in the DOM.
 *
 * Strings are translated through the i18n provider; React elements are
 * rendered as-is.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/title Title documentation}
 *
 * @example
 * import { Title } from "@/components/admin";
 *
 * const Dashboard = () => (
 *   <>
 *     <Title title="Dashboard" />
 *     <div>Welcome!</div>
 *   </>
 * );
 */
function Title({ title, defaultTitle, className }: TitleProps) {
  const translate = useTranslate();
  const [target, setTarget] = useState<HTMLElement | null>(() =>
    typeof document !== "undefined"
      ? document.getElementById(TITLE_PORTAL_ID)
      : null,
  );

  useEffect(() => {
    setTarget((current) => {
      const isInTheDom =
        typeof document !== "undefined" &&
        current !== null &&
        document.body.contains(current);
      if (current && isInTheDom) return current;
      return typeof document !== "undefined"
        ? document.getElementById(TITLE_PORTAL_ID)
        : null;
    });
  }, []);

  if (!target) return null;

  const value = title ?? defaultTitle ?? "";
  const node =
    typeof value === "string" ? (
      <h1 className={cn("text-lg font-semibold truncate", className)}>
        {value ? translate(value, { _: value }) : ""}
      </h1>
    ) : (
      value
    );

  return createPortal(node, target);
}

export { Title, type TitleProps };
