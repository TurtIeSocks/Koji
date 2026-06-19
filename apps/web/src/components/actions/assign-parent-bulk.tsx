import { useState } from "react";
import { FormProvider, useForm } from "react-hook-form";
import {
  useDataProvider,
  useNotify,
  useRefresh,
  useListContext,
  useUnselectAll,
} from "shadmin-core";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ReferenceInput, AutocompleteInput } from "@/components/admin";

interface FormValues {
  parent: number | null;
}

export function AssignParentBulkButton() {
  const [open, setOpen] = useState(false);
  const { selectedIds } = useListContext();
  const unselectAll = useUnselectAll("geofence");
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const refresh = useRefresh();

  const form = useForm<FormValues>({ defaultValues: { parent: null } });

  const handleOpen = () => {
    form.reset({ parent: null });
    setOpen(true);
  };

  const handleClose = () => setOpen(false);

  const handleSave = form.handleSubmit(async (values) => {
    try {
      await dataProvider.updateMany("geofence", {
        ids: selectedIds,
        data: { parent: values.parent ?? null },
      });
      notify(`Parent assigned to ${selectedIds.length} geofence(s)`, {
        type: "info",
      });
    } catch {
      notify("Failed to assign parent", { type: "error" });
    } finally {
      unselectAll();
      refresh();
      setOpen(false);
    }
  });

  const handleClearParent = async () => {
    try {
      await dataProvider.updateMany("geofence", {
        ids: selectedIds,
        data: { parent: null },
      });
      notify(`Parent cleared from ${selectedIds.length} geofence(s)`, {
        type: "info",
      });
    } catch {
      notify("Failed to clear parent", { type: "error" });
    } finally {
      unselectAll();
      refresh();
      setOpen(false);
    }
  };

  return (
    <>
      <Button size="sm" variant="secondary" type="button" onClick={handleOpen}>
        Assign Parent
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent showCloseButton>
          <DialogHeader>
            <DialogTitle>
              Assign Parent to {selectedIds.length} geofence(s)
            </DialogTitle>
          </DialogHeader>
          <FormProvider {...form}>
            <form onSubmit={handleSave} className="flex flex-col gap-4">
              <ReferenceInput source="parent" reference="geofence">
                <AutocompleteInput />
              </ReferenceInput>
              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleClearParent}
                >
                  Clear Parent
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleClose}
                >
                  Cancel
                </Button>
                <Button type="submit">Save</Button>
              </DialogFooter>
            </form>
          </FormProvider>
        </DialogContent>
      </Dialog>
    </>
  );
}
