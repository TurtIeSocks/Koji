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
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ReferenceArrayInput, AutocompleteArrayInput } from "@/components/admin";

interface FormValues {
  projects: number[];
}

export function AssignProjectsBulkButton() {
  const [open, setOpen] = useState(false);
  const { selectedIds } = useListContext();
  const unselectAll = useUnselectAll("geofence");
  const dataProvider = useDataProvider();
  const notify = useNotify();
  const refresh = useRefresh();

  const form = useForm<FormValues>({ defaultValues: { projects: [] } });

  const handleOpen = () => {
    form.reset({ projects: [] });
    setOpen(true);
  };

  const handleClose = () => setOpen(false);

  const handleSave = form.handleSubmit(async (values) => {
    try {
      await dataProvider.updateMany("geofence", {
        ids: selectedIds,
        data: { projects: values.projects },
      });
      notify(`Projects assigned to ${selectedIds.length} geofence(s)`, {
        type: "info",
      });
    } catch {
      notify("Failed to assign projects", { type: "error" });
    } finally {
      unselectAll();
      refresh();
      setOpen(false);
    }
  });

  return (
    <>
      <Button size="sm" variant="secondary" type="button" onClick={handleOpen}>
        Assign Projects
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent showCloseButton>
          <DialogHeader>
            <DialogTitle>
              Assign Projects to {selectedIds.length} geofence(s)
            </DialogTitle>
            <DialogDescription>
              This replaces the project list on each selected geofence. Projects
              not chosen here will be unlinked.
            </DialogDescription>
          </DialogHeader>
          <FormProvider {...form}>
            <form onSubmit={handleSave} className="flex flex-col gap-4">
              <ReferenceArrayInput source="projects" reference="project">
                <AutocompleteArrayInput />
              </ReferenceArrayInput>
              <DialogFooter>
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
