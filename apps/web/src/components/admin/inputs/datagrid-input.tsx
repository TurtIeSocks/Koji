import type { ReactElement, ReactNode } from "react";
import { useCallback, useMemo } from "react";
import type {
  ChoicesProps,
  Identifier,
  InputProps,
  ListControllerResult,
  RaRecord,
} from "shadmin-core";
import { ListContextProvider, useChoicesContext, useInput } from "shadmin-core";

import { cn } from "@/lib/utils";
import { DataTable } from "@/components/admin/list/data-table";
import { InputHelperText } from "@/components/admin/common/input-helper-text";

type DatagridInputProps = Omit<
  InputProps,
  "fullWidth" | "readOnly" | "disabled" | "source"
> &
  Omit<ChoicesProps, "disableValue"> & {
    children?: ReactNode;
    /** Optional pagination element rendered below the table. */
    pagination?: ReactElement | false;
    className?: string;
    label?: ReactNode;
    helperText?: ReactNode;
    source?: string;
    resource?: string;
    reference?: string;
    choices?: RaRecord[];
  };

/**
 * Form input that displays a list of records and lets the user select multiple
 * rows via row checkboxes. The form value is an array of selected record ids.
 *
 * Typically used as a child of `<ReferenceArrayInput>`, where it gets its
 * choices from the choices context. Can also be used directly with a `choices`
 * prop for static data.
 *
 * @experimental Upstream `<DatagridInput>` is marked WIP. This simplified port
 * supports row selection and pagination but does not yet integrate filter
 * contexts or the create-suggestion flow.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/datagrid-input DatagridInput documentation}
 */
function DatagridInput<RecordType extends RaRecord = RaRecord>(
  props: DatagridInputProps,
) {
  const {
    children,
    choices,
    className,
    pagination,
    source: sourceProp,
    resource: resourceProp,
    helperText,
    label: _label,
    reference: _reference,
    ...rest
  } = props;

  const {
    availableChoices,
    error: fetchError,
    source,
    ...choicesContext
  } = useChoicesContext({
    choices,
    resource: resourceProp,
    source: sourceProp,
  });

  const { field, fieldState } = useInput({
    ...rest,
    source: source ?? sourceProp ?? "",
  });

  const fieldValue = useMemo<Identifier[]>(
    () => (Array.isArray(field.value) ? field.value : []),
    [field.value],
  );

  const onSelect = useCallback(
    (ids: Identifier[]) => {
      field.onChange(ids);
    },
    [field],
  );

  const onToggleItem = useCallback(
    (id: Identifier) => {
      if (fieldValue.includes(id)) {
        field.onChange(fieldValue.filter((item) => item !== id));
      } else {
        field.onChange([...fieldValue, id]);
      }
    },
    [field, fieldValue],
  );

  const onUnselectItems = useCallback(() => {
    field.onChange([]);
  }, [field]);

  const onSelectAll = useCallback(() => {
    if (!availableChoices) return;
    field.onChange(availableChoices.map((choice) => choice.id));
  }, [availableChoices, field]);

  const listContext = useMemo(
    () => ({
      ...choicesContext,
      data: availableChoices,
      total: availableChoices?.length,
      meta: undefined,
      error: null,
      onSelect,
      onSelectAll,
      onToggleItem,
      onUnselectItems,
      selectedIds: fieldValue,
      resource: resourceProp ?? choicesContext.resource,
    }),
    [
      availableChoices,
      // biome-ignore lint/correctness/useExhaustiveDependencies: choicesContext is a rest-spread of useChoicesContext() recreated each render; it is intentionally spread into listContext and cannot be wrapped in its own useMemo at this call site.
      choicesContext,
      fieldValue,
      onSelect,
      onSelectAll,
      onToggleItem,
      onUnselectItems,
      resourceProp,
    ],
  );

  return (
    <div
      className={cn(
        "ra-input",
        source ? `ra-input-${source}` : undefined,
        className,
      )}
      data-slot="datagrid-input"
    >
      <ListContextProvider value={listContext as ListControllerResult}>
        {!fieldState.error && !fetchError && (
          <>
            <DataTable<RecordType>
              bulkActionButtons={false}
              // Pass an empty fragment so `hasBulkActions` becomes `true`
              // (which enables the row checkbox column) while avoiding the
              // default bulk-actions toolbar (delete/export), which would be
              // out of place inside a form input.
              // biome-ignore lint/complexity/noUselessFragments: deliberate non-null sentinel that flips hasBulkActions on without rendering a toolbar.
              bulkActionsToolbar={<></>}
            >
              {children}
            </DataTable>
            {pagination !== false && pagination ? pagination : null}
          </>
        )}
        <InputHelperText
          helperText={
            fieldState.error?.message ?? fetchError?.message ?? helperText
          }
        />
      </ListContextProvider>
    </div>
  );
}

export { DatagridInput, type DatagridInputProps };
