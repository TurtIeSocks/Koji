import type { ComponentProps, ReactNode } from "react";
import { useEffect } from "react";
import { Link, useMatch, useNavigate } from "react-router";
import { useBasename, useTranslate } from "shadmin-core";
import {
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { KeyboardShortcut } from "@/components/admin/common/keyboard-shortcut";

const KEY_TO_PROP: Record<
  string,
  "metaKey" | "ctrlKey" | "shiftKey" | "altKey"
> = {
  meta: "metaKey",
  cmd: "metaKey",
  ctrl: "ctrlKey",
  shift: "shiftKey",
  alt: "altKey",
  option: "altKey",
};

const isMac =
  typeof navigator !== "undefined" && /Mac/i.test(navigator.platform);

function parseShortcut(shortcut: string) {
  const parts = shortcut
    .toLowerCase()
    .split("+")
    .map((s) => s.trim());
  const modifiers = new Set<string>();
  let key = "";
  for (const part of parts) {
    if (part === "mod") {
      modifiers.add(isMac ? "metaKey" : "ctrlKey");
    } else if (KEY_TO_PROP[part]) {
      modifiers.add(KEY_TO_PROP[part]);
    } else {
      key = part;
    }
  }
  return { modifiers, key };
}

type MenuItemLinkProps = {
  /**
   * Target path for the link. Compared against the current location to
   * derive the active state. Accepts a string or a react-router `To` object.
   */
  to:
    | string
    | { pathname: string; search?: string; hash?: string; state?: unknown };
  /**
   * The text rendered alongside `leftIcon`. Strings are auto-translated via
   * `useTranslate` (with the string itself as the fallback). Pass a ReactNode
   * to render custom content without translation lookup.
   */
  primaryText: ReactNode;
  /**
   * Icon rendered before `primaryText`. Pass a `lucide-react` icon (or any
   * ReactNode) sized to match other sidebar items.
   */
  leftIcon?: ReactNode;
  /**
   * Extra CSS class appended to the underlying `<SidebarMenuButton>`.
   */
  className?: string;
  /**
   * Click handler invoked after the default navigation. The default
   * `<AppSidebar>` uses this to close the mobile drawer.
   */
  onClick?: () => void;
  /**
   * Keyboard shortcut string (e.g. `"mod+k"`, `"shift+/"`, `"meta+s"`).
   * When pressed anywhere on the page, navigates to `to`.
   * `mod` normalizes to `metaKey` on macOS, `ctrlKey` elsewhere.
   */
  keyboardShortcut?: string;
  /**
   * Visual representation of the keyboard shortcut rendered after the label.
   * Defaults to `<KeyboardShortcut keyboardShortcut={keyboardShortcut} />`.
   */
  keyboardShortcutRepresentation?: ReactNode;
  /**
   * Extra props forwarded to the `<Tooltip>` wrapping the menu item.
   * Applies to both collapsed-sidebar and expanded-sidebar (if tooltipProps
   * is provided) modes.
   */
  tooltipProps?: Omit<ComponentProps<typeof Tooltip>, "children">;
};

/**
 * A clickable sidebar entry that navigates to a route.
 *
 * `<MenuItemLink>` renders a sidebar item with an icon and a label that
 * navigates via react-router. It is the building block for custom sidebar
 * menus and is used internally by `<DashboardMenuItem>`.
 *
 * The active state is derived from `useMatch`; when the current pathname
 * starts with `to`, the item is highlighted.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/menu-item-link MenuItemLink documentation}
 *
 * @example
 * import { Settings } from "lucide-react";
 * import { MenuItemLink } from "@/components/admin";
 *
 * const SettingsItem = () => (
 *   <MenuItemLink
 *     to="/settings"
 *     primaryText="Settings"
 *     leftIcon={<Settings />}
 *   />
 * );
 */
function MenuItemLink({
  to,
  primaryText,
  leftIcon,
  className,
  onClick,
  keyboardShortcut,
  keyboardShortcutRepresentation,
  tooltipProps,
}: MenuItemLinkProps) {
  const translate = useTranslate();
  const basename = useBasename();
  const navigate = useNavigate();
  const finalText =
    typeof primaryText === "string"
      ? translate(primaryText, { _: primaryText })
      : primaryText;
  const toPathname = typeof to === "string" ? to : to.pathname;
  const match = useMatch({
    path: toPathname,
    end: toPathname === `${basename}/`,
  });
  const { open, openMobile, setOpenMobile } = useSidebar();
  const handleClick = () => {
    if (openMobile) {
      setOpenMobile(false);
    }
    onClick?.();
  };

  useEffect(() => {
    if (!keyboardShortcut) return;
    const { modifiers, key } = parseShortcut(keyboardShortcut);
    const handler = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() !== key) return;
      for (const mod of ["metaKey", "ctrlKey", "shiftKey", "altKey"] as const) {
        const required = modifiers.has(mod);
        if (e[mod] !== required) return;
      }
      e.preventDefault();
      navigate(to);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [keyboardShortcut, to, navigate]);

  const shortcutNode =
    keyboardShortcut !== undefined
      ? (keyboardShortcutRepresentation ?? (
          <KeyboardShortcut keyboardShortcut={keyboardShortcut} />
        ))
      : null;

  const button = (
    <SidebarMenuButton asChild isActive={!!match} className={className}>
      <Link to={to} onClick={handleClick}>
        {leftIcon}
        {finalText}
        {shortcutNode && <span className="ml-auto">{shortcutNode}</span>}
      </Link>
    </SidebarMenuButton>
  );

  return (
    <SidebarMenuItem>
      {open && !tooltipProps ? (
        button
      ) : (
        <Tooltip {...tooltipProps}>
          <TooltipTrigger asChild>{button}</TooltipTrigger>
          <TooltipContent side="right">{finalText}</TooltipContent>
        </Tooltip>
      )}
    </SidebarMenuItem>
  );
}

export { MenuItemLink, type MenuItemLinkProps };
