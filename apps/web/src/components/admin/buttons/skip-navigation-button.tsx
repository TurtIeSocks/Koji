import * as React from "react";
import { useTranslate } from "shadmin-core";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { resolveLabel } from "@/lib/resolve-label";

type SkipNavigationButtonProps = Omit<
  React.ComponentProps<typeof Button>,
  "onClick"
> & {
  label?: string;
  ref?: React.Ref<HTMLButtonElement>;
};

/**
 * Accessibility skip-navigation link that jumps focus to the page's main content.
 *
 * Renders a button that is visually hidden until it receives keyboard focus.
 * Looks for an element with `id="main-content"` and moves focus to it.
 *
 * Place this component at the top of the page (e.g., in the layout) so it is the
 * first focusable element when navigating with the keyboard.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/skip-navigation-button SkipNavigationButton documentation}
 *
 * @example
 * import { SkipNavigationButton } from '@/components/admin';
 *
 * const Layout = ({ children }) => (
 *   <>
 *     <SkipNavigationButton />
 *     <main id="main-content">{children}</main>
 *   </>
 * );
 */
function SkipNavigationButton(props: SkipNavigationButtonProps) {
  const { label = "ra.navigation.skip_nav", className, ref, ...rest } = props;
  const translate = useTranslate();
  const translatedLabel = resolveLabel(label, translate, "Skip to content");

  return (
    <Button
      type="button"
      onClick={skipToContent}
      ref={ref}
      className={cn(
        "skip-nav-button fixed left-4 -top-24 z-5000 focus:top-4",
        "transition-[top,opacity] duration-200 ease-in focus:ease-out",
        "hover:opacity-80",
        className,
      )}
      {...rest}
    >
      {translatedLabel}
    </Button>
  );
}

const skipToContent = () => {
  if (typeof document === "undefined") return;
  const element = document.getElementById("main-content");

  if (!element) {
    if (process.env.NODE_ENV !== "production") {
      console.warn(
        'No element with id "main-content" was found. Ensure the element that contains your main content has an id of "main-content".',
      );
    }
    return;
  }

  element.setAttribute("tabIndex", "-1");
  element.focus();
  element.blur();
  element.removeAttribute("tabIndex");
};

export { SkipNavigationButton, type SkipNavigationButtonProps };
