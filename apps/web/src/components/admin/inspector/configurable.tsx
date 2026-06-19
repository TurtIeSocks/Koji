import {
  useRef,
  useEffect,
  useState,
  cloneElement,
  isValidElement,
  type ReactElement,
  type ReactNode,
} from "react";
import {
  usePreferencesEditor,
  PreferenceKeyContextProvider,
  useTranslate,
} from "shadmin-core";
import { Settings } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * Wrap any component to make it editable through the {@link Inspector} panel.
 *
 * When the preferences edit mode is enabled (typically via the
 * {@link InspectorButton}), users see a yellow outline around the wrapped
 * children plus a small Settings icon on hover. Clicking the icon opens the
 * provided `editor` element inside the {@link Inspector}.
 *
 * Creates a {@link PreferenceKeyContextProvider} so that both the child and the
 * editor can read and write user preferences scoped to `preferences.${preferenceKey}`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/inspector Inspector documentation}
 *
 * @example
 * import { Configurable, FieldsSelector } from "@/components/admin";
 *
 * const ConfigurableTextBlock = ({ preferenceKey = "TextBlock", ...props }) => (
 *   <Configurable editor={<FieldsSelector />} preferenceKey={preferenceKey}>
 *     <TextBlock {...props} />
 *   </Configurable>
 * );
 */
function Configurable(props: ConfigurableProps) {
  const {
    children,
    editor,
    preferenceKey,
    openButtonLabel = "ra.configurable.customize",
    className,
  } = props;

  const prefixedPreferenceKey = `preferences.${preferenceKey}`;
  const preferencesEditorContext = usePreferencesEditor();
  const hasPreferencesEditorContext = !!preferencesEditorContext;

  const translate = useTranslate();

  const {
    isEnabled,
    setEditor,
    preferenceKey: currentPreferenceKey,
    setPreferenceKey,
  } = preferencesEditorContext || {};

  const isEditorOpen = prefixedPreferenceKey === currentPreferenceKey;
  const editorOpenRef = useRef(isEditorOpen);
  const [isHovered, setIsHovered] = useState(false);

  useEffect(() => {
    editorOpenRef.current = isEditorOpen;
  }, [isEditorOpen]);

  useEffect(() => {
    return () => {
      if (!editorOpenRef.current) return;
      setPreferenceKey?.(null);
      setEditor?.(null);
    };
  }, [setEditor, setPreferenceKey]);

  if (!hasPreferencesEditorContext) {
    return <>{children}</>;
  }

  const handleOpenEditor = () => {
    // Include the editor key as React key to force destroy and mount
    // when switching between two identical editors with different keys.
    // Otherwise the editor would see a prop update and its useStore would
    // return one tick later, breaking uncontrolled inputs in the editor.
    setEditor!(
      isValidElement(editor)
        ? cloneElement(editor as ReactElement<{ preferenceKey?: string }>, {
            preferenceKey: prefixedPreferenceKey,
            key: prefixedPreferenceKey,
          })
        : editor,
    );
    setPreferenceKey!(prefixedPreferenceKey);
  };

  const showButton = isEnabled && (isHovered || isEditorOpen);

  return (
    <PreferenceKeyContextProvider value={prefixedPreferenceKey}>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: presentational wrapper around app content; mouse/focus handlers only reveal the nested keyboard-accessible Customize button (hover + focus-within affordance), they are not a primary interaction */}
      <span
        className={cn(
          "relative inline-block",
          isEnabled &&
            "outline outline-2 outline-offset-2 outline-amber-500/70 hover:outline-amber-500 transition-[outline]",
          isEnabled && isEditorOpen && "outline-amber-500",
          className,
        )}
        onMouseEnter={isEnabled ? () => setIsHovered(true) : undefined}
        onMouseLeave={isEnabled ? () => setIsHovered(false) : undefined}
        onFocus={isEnabled ? () => setIsHovered(true) : undefined}
        onBlur={
          isEnabled
            ? (e) => {
                if (!e.currentTarget.contains(e.relatedTarget as Node)) {
                  setIsHovered(false);
                }
              }
            : undefined
        }
      >
        {children}
        {showButton && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={translate(openButtonLabel, { _: "Customize" })}
            title={translate(openButtonLabel, { _: "Customize" })}
            onClick={handleOpenEditor}
            className={cn(
              "absolute -top-3 -right-3 z-10 size-6 rounded-full",
              "bg-amber-500 text-amber-50 hover:bg-amber-500/90 hover:text-amber-50 shadow-sm",
            )}
          >
            <Settings className="size-3.5" />
          </Button>
        )}
      </span>
    </PreferenceKeyContextProvider>
  );
}

interface ConfigurableProps {
  /** The element that becomes configurable. */
  children: ReactNode;
  /**
   * The editor element rendered inside the Inspector when this configurable is
   * selected. The editor is cloned with `preferenceKey` injected, so editor
   * components like {@link FieldsSelector} read preferences from the same
   * namespace as the configurable child.
   */
  editor: ReactElement;
  /**
   * Identifies this configurable in the preferences store. The Inspector
   * stores user changes under `preferences.${preferenceKey}`.
   */
  preferenceKey: string;
  /** Translation key for the open-editor button label. */
  openButtonLabel?: string;
  /** Extra classes appended to the wrapper element. */
  className?: string;
}

export { Configurable, type ConfigurableProps };
