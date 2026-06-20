import { FormProvider, useForm } from "react-hook-form";
import { ResourceContextProvider } from "shadmin-core";
import { Button } from "@/components/ui/button";
import { Stepper } from "@/components/import/stepper";
import { SourceStep } from "./steps/source-step";
import { IMPORT_STEPS, useImportStep } from "./wizard-context";

/** The import wizard page (`/import`). One RHF form hosts all steps; the
 *  imported features + per-feature assignments live in `features` (an RHF field
 *  array, populated by the Source step). B5–B7 fill the later step panels. */
function ImportWizard() {
  const methods = useForm({ defaultValues: { features: [] as unknown[] } });
  const { active, next, back } = useImportStep();

  return (
    <ResourceContextProvider value="geofence">
      <FormProvider {...methods}>
        <div className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
          <h1 className="text-xl font-semibold">Import</h1>
          <Stepper steps={[...IMPORT_STEPS]} active={active} />
          <div className="min-h-[40vh]">
            {active === 0 && <SourceStep onLoaded={next} />}
            {active === 1 && (
              <p className="text-muted-foreground">Map &amp; Name (B5)</p>
            )}
            {active === 2 && <p className="text-muted-foreground">Assign (B6)</p>}
            {active === 3 && <p className="text-muted-foreground">Review (B7)</p>}
          </div>
          <div className="flex justify-between">
            <Button variant="outline" onClick={back} disabled={active === 0}>
              Back
            </Button>
            <Button
              onClick={next}
              disabled={active === IMPORT_STEPS.length - 1}
            >
              Next
            </Button>
          </div>
        </div>
      </FormProvider>
    </ResourceContextProvider>
  );
}

export { ImportWizard };
export default ImportWizard;
