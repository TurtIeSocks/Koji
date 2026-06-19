import { useState, useEffect, useRef } from "react";
import {
  useStore,
  usePreferencesEditor,
  useTranslate,
  useRemoveItemsFromStore,
  PreferenceKeyContextProvider,
} from "shadmin-core";
import { Trash2, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

import { InspectorRoot } from "@/components/admin/inspector/inspector-root";

/**
 * Floating panel that renders the editor of the currently selected
 * {@link Configurable}.
 *
 * Reads `editor`, `title`, `preferenceKey`, and the `isEnabled` flag from
 * {@link usePreferencesEditor}. When `isEnabled` is `false`, this component
 * renders nothing. The Close button calls `disable()`; the Trash button
 * removes every preference under the current key prefix.
 *
 * The panel is draggable by its title bar and its position is persisted via
 * `useStore('ra.inspector.position')`.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/inspector Inspector documentation}
 */
function Inspector({ className }: InspectorProps = {}) {
  const { isEnabled, disable, title, titleOptions, editor, preferenceKey } =
    usePreferencesEditor();
  const removeItems = useRemoveItemsFromStore(preferenceKey);
  const translate = useTranslate();
  const [version, setVersion] = useState(0);

  // Persist position across sessions. Default: top-right corner.
  // 320 = w-80 panel width; 8 = margin.
  const [dialogPosition, setDialogPosition] = useStore(
    "ra.inspector.position",
    {
      x:
        typeof document !== "undefined"
          ? document.body.clientWidth - 320 - 8
          : 8,
      y: 8,
    },
  );

  // isDragging ref tracks whether this specific drag originated on the title bar.
  const isDragging = useRef(false);

  // Store the offset between the pointer and the dialog's top-left corner at
  // drag-start so we can compute the new position on drag-end.
  const [clickPosition, setClickPosition] = useState<
    { x: number; y: number } | undefined
  >();

  // React state for visibility during drag (avoids imperative classList mutation).
  const [isHidden, setIsHidden] = useState(false);

  const handleDragStart = (e: React.DragEvent<HTMLDivElement>) => {
    // Only allow dragging when the pointer is directly over the title bar.
    // elementFromPoint is used because the drag event fires on the root element
    // but we want to restrict the handle to the title span.
    const draggedElement = document?.elementFromPoint(e.clientX, e.clientY);
    if (draggedElement?.id !== "inspector-dialog-title") {
      return;
    }

    isDragging.current = true;
    e.dataTransfer.effectAllowed = "move";
    // Tag the transfer so our dragover filter can identify inspector drags.
    e.dataTransfer.setData("inspector", "");

    // Hide the panel on the next tick — doing it synchronously prevents the
    // browser from using the current element as the drag ghost image.
    setTimeout(() => setIsHidden(true), 0);

    setClickPosition({
      x: e.clientX - dialogPosition.x,
      y: e.clientY - dialogPosition.y,
    });
  };

  const handleDragEnd = (e: React.DragEvent<HTMLDivElement>) => {
    if (isDragging.current && clickPosition) {
      setDialogPosition({
        x: e.clientX - clickPosition.x,
        y: e.clientY - clickPosition.y,
      });
      setIsHidden(false);
      isDragging.current = false;
    }
  };

  // Prevent the browser's "snap back" animation when the inspector is dropped
  // anywhere on the page. We intercept dragover only for inspector drags.
  useEffect(() => {
    if (!isEnabled) return;
    const handleDragover = (e: DragEvent) => {
      if (e.dataTransfer?.types.includes("inspector")) {
        e.preventDefault();
      }
    };
    document?.addEventListener("dragover", handleDragover);
    return () => {
      document?.removeEventListener("dragover", handleDragover);
    };
  }, [isEnabled]);

  // Clamp position whenever the window is resized so the panel never escapes
  // the viewport. Also runs once on mount (when isEnabled first becomes true).
  useEffect(() => {
    if (!isEnabled) return;
    const clamp = () => {
      window?.requestAnimationFrame(() => {
        setDialogPosition((p) => ({
          x: Math.min(p.x, document.body.clientWidth - 320 - 8),
          y: Math.min(p.y, window.innerHeight - 50),
        }));
      });
    };
    clamp();
    window?.addEventListener("resize", clamp);
    return () => {
      window?.removeEventListener("resize", clamp);
    };
  }, [isEnabled, setDialogPosition]);

  const handleReset = () => {
    removeItems();
    // force re-mount of the editor so it picks up default values
    setVersion((v) => v + 1);
  };

  if (!isEnabled) return null;

  const resolvedTitle = title
    ? translate(title, { _: "Inspector", ...titleOptions })
    : translate("ra.configurable.inspector.title", { _: "Inspector" });

  return (
    <div
      role="dialog"
      aria-label={resolvedTitle}
      draggable
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      className={cn(
        "fixed z-50 w-80 max-w-[calc(100vw-2rem)]",
        "rounded-md border bg-popover text-popover-foreground shadow-lg",
        isHidden && "hidden",
        className,
      )}
      style={{ left: dialogPosition.x, top: dialogPosition.y }}
    >
      {/* cursor-move on the header signals the drag affordance to users */}
      <div className="flex items-center justify-between border-b px-3 py-2 cursor-move">
        <span
          id="inspector-dialog-title"
          className="text-xs uppercase font-semibold text-muted-foreground tracking-wide"
        >
          {resolvedTitle}
        </span>
        {/* Portal slot — sub-components (DatagridEditor, SimpleFormEditor, etc.)
            render their toolbar buttons into this span via a portal. */}
        <span id="inspector-toolbar" />
        <div className="flex items-center gap-1">
          {preferenceKey && (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-7"
              aria-label={translate("ra.action.remove", { _: "Remove" })}
              onClick={handleReset}
            >
              <Trash2 className="size-4" />
            </Button>
          )}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            aria-label={translate("ra.action.close", { _: "Close" })}
            onClick={disable}
          >
            <X className="size-4" />
          </Button>
        </div>
      </div>
      <div className="p-3 max-h-[75vh] overflow-y-auto" key={version}>
        <PreferenceKeyContextProvider value={preferenceKey}>
          {editor ?? <InspectorRoot />}
        </PreferenceKeyContextProvider>
      </div>
    </div>
  );
}

Inspector.displayName = "Inspector";

interface InspectorProps {
  /** Extra classes appended to the inspector card. */
  className?: string;
}

export { Inspector, type InspectorProps };
