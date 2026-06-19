"use client";

import type { ElementType, ReactElement } from "react";
import { FieldTitle } from "shadmin-core";
import { cn } from "@/lib/utils";

/**
 * Wraps a field with an auto-generated label derived from the field's `source`
 * or explicit `label` prop.
 *
 * The label is suppressed when:
 * - `label={false}` is on this component, or
 * - the child's own `label` prop is `false`, or
 * - the child is a native DOM element (e.g. `<span>`), or
 * - the child is already a `<Labeled>` (prevents double-labeling).
 *
 * Used automatically by `TabbedShowLayout.Tab` to label every field,
 * and can be used manually in custom Show views.
 *
 * @see {@link https://marmelab.com/react-admin/Labeled.html}
 *
 * @example
 * // Explicit label
 * <Labeled label="Author name">
 *   <TextField source="author" />
 * </Labeled>
 *
 * @example
 * // Auto-label from source
 * <Labeled>
 *   <TextField source="published_at" />
 * </Labeled>
 */
function Labeled({
  children,
  className,
  component: Wrapper = "div",
  fullWidth,
  htmlFor,
  isRequired,
  label,
  resource,
  source,
}: LabeledProps) {
  // Suppress label when any condition signals "no label".
  const childLabel = (children as ReactElement<{ label?: React.ReactNode }>)
    ?.props?.label;
  const childType = (children as ReactElement)?.type;
  const showLabel =
    label !== false &&
    childLabel !== false &&
    typeof childType !== "string" &&
    (childType as { displayName?: string })?.displayName !== "Labeled";

  return (
    <Wrapper
      className={cn(
        "inline-flex flex-col gap-1",
        fullWidth && "w-full",
        className,
      )}
    >
      {showLabel && (
        <label
          htmlFor={htmlFor}
          className="text-muted-foreground text-xs font-medium"
        >
          <FieldTitle
            label={label ?? childLabel}
            source={
              source ??
              (children as ReactElement<{ source?: string }>)?.props?.source
            }
            resource={resource}
            isRequired={isRequired}
          />
        </label>
      )}
      {children}
    </Wrapper>
  );
}

Labeled.displayName = "Labeled";

interface LabeledProps {
  children: ReactElement;
  className?: string;
  /** Override the wrapper element type. Defaults to `"div"`. */
  component?: ElementType;
  /** Expand the component to fill its container's width. */
  fullWidth?: boolean;
  /** When provided, sets the `for` attribute on the rendered `<label>` so clicking it focuses the associated input. */
  htmlFor?: string;
  /** Whether to show a required indicator. */
  isRequired?: boolean;
  /** Set to `false` to hide the label entirely. */
  label?: React.ReactNode;
  /** Override the resource for label translation. */
  resource?: string;
  /** Override the source for label translation. */
  source?: string;
}

export { Labeled, type LabeledProps };
