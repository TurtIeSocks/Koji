import { usePreference, useTranslate } from "shadmin-core";

import { Button } from "@/components/ui/button";

import { FieldToggle } from "@/components/admin/inspector/field-toggle";

interface FieldsSelectorProps {
  /**
   * Preference key for the array of selected field indexes.
   *
   * @default "columns"
   */
  name?: string;
  /**
   * Preference key for the array of available fields.
   *
   * @default "availableColumns"
   */
  availableName?: string;
}

interface SelectableField {
  /** Stable identifier used when storing the user's selection / order. */
  index: string;
  /** Field source path (used for i18n via {@link FieldTitle}). */
  source: string;
  /** Optional human label, overrides the source-based label. */
  label?: React.ReactNode;
}

/**
 * Editor body that lets the user toggle and reorder fields, persisting the
 * choices to the preferences store.
 *
 * Intended to be used as the `editor` prop of a {@link Configurable}, where it
 * receives a `preferenceKey` (injected automatically by `Configurable.cloneElement`)
 * and reads two preferences from that namespace:
 *
 * - `availableName` (default `"availableColumns"`): the full list of selectable
 *   fields, as `{ index, source, label? }[]`.
 * - `name` (default `"columns"`): the indexes of the fields the user has
 *   selected, in display order.
 *
 * It also reads an `omit` preference to filter out columns hidden by default.
 *
 * Renders a list of {@link FieldToggle} rows (drag-and-drop reorderable) plus
 * Hide All / Show All buttons.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/inspector Inspector documentation}
 */
function FieldsSelector({
  name = "columns",
  availableName = "availableColumns",
}: FieldsSelectorProps) {
  const translate = useTranslate();

  const [availableFields = [], setAvailableFields] = usePreference<
    SelectableField[]
  >(availableName, []);
  const [omit] = usePreference<string[]>("omit", []);

  const [fields = [], setFields] = usePreference<string[]>(
    name,
    availableFields
      .filter((field) => !omit?.includes(field.source))
      .map((field) => field.index),
  );

  const handleToggle = (fieldIndex: string, checked: boolean) => {
    if (checked) {
      // add the column at its natural position (preserving availableFields order)
      setFields(
        availableFields
          .filter(
            (field) =>
              field.index === fieldIndex || fields.includes(field.index),
          )
          .map((field) => field.index),
      );
    } else {
      setFields(fields.filter((index) => index !== fieldIndex));
    }
  };

  const handleMove = (
    index1: string | number,
    index2: string | number | null,
  ) => {
    const index1Pos = availableFields.findIndex(
      (field) => field.index === String(index1),
    );
    const index2Pos = availableFields.findIndex(
      (field) => field.index === String(index2),
    );
    if (index1Pos === -1 || index2Pos === -1) {
      return;
    }
    let newAvailableFields: SelectableField[];
    if (index1Pos > index2Pos) {
      newAvailableFields = [
        ...availableFields.slice(0, index2Pos),
        availableFields[index1Pos],
        ...availableFields.slice(index2Pos, index1Pos),
        ...availableFields.slice(index1Pos + 1),
      ];
    } else {
      newAvailableFields = [
        ...availableFields.slice(0, index1Pos),
        ...availableFields.slice(index1Pos + 1, index2Pos + 1),
        availableFields[index1Pos],
        ...availableFields.slice(index2Pos + 1),
      ];
    }
    setAvailableFields(newAvailableFields);
    setFields(
      newAvailableFields
        .filter((field) => fields.includes(field.index))
        .map((field) => field.index),
    );
  };

  const handleHideAll = () => setFields([]);
  const handleShowAll = () =>
    setFields(availableFields.map((field) => field.index));

  return (
    <div className="pt-1">
      <ul className="m-0 list-none p-0">
        {availableFields.map((field) => (
          <FieldToggle
            key={field.index}
            source={field.source}
            label={field.label}
            index={field.index}
            selected={fields.includes(field.index)}
            onToggle={(checked: boolean) => handleToggle(field.index, checked)}
            onMove={handleMove}
          />
        ))}
      </ul>
      <div className="mt-2 flex justify-between">
        <Button type="button" size="sm" variant="ghost" onClick={handleHideAll}>
          {translate("ra.configurable.inspector.hideAll", { _: "Hide All" })}
        </Button>
        <Button type="button" size="sm" variant="ghost" onClick={handleShowAll}>
          {translate("ra.configurable.inspector.showAll", { _: "Show All" })}
        </Button>
      </div>
    </div>
  );
}

export { FieldsSelector, type FieldsSelectorProps, type SelectableField };
