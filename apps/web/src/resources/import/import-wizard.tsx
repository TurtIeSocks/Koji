import { useEffect, useState } from "react";
import { useBlocker } from "react-router";
import { FormProvider, useForm } from "react-hook-form";
import { ResourceContextProvider } from "shadmin-core";
import { Button } from "@/components/ui/button";
import { Stepper } from "@/components/import/stepper";
import { SourceStep } from "./steps/source-step";
import { MapNameStep } from "./steps/map-name-step";
import { AssignStep } from "./steps/assign-step";
import { ReviewStep } from "./steps/review-step";
import { IMPORT_STEPS, useImportStep } from "./wizard-context";

/** The import wizard page (`/import`). One RHF form hosts all steps; the
 *  imported features + per-feature assignments live in `features` (an RHF field
 *  array, populated by the Source step). B5–B7 fill the later step panels. */
function ImportWizard() {
  const methods = useForm({ defaultValues: { features: [] as unknown[] } });
  const { active, next, back } = useImportStep();
  const [committed, setCommitted] = useState(false);
  const isDirty = methods.formState.isDirty;

  // --- Leave-guard: tab close / reload ----------------------------------
  useEffect(() => {
    if (!isDirty || committed) return;
    const beforeunload = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
      return "";
    };
    window.addEventListener("beforeunload", beforeunload);
    return () => window.removeEventListener("beforeunload", beforeunload);
  }, [isDirty, committed]);

  // --- Leave-guard: in-app navigation (react-router data router) --------
  // The Admin shell mounts a data router, so useBlocker is available. It is
  // called unconditionally (hooks must never be conditional); the condition
  // returns false unless there is unsaved, uncommitted work, so the blocker
  // stays inert until the form is dirty.
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      isDirty && !committed && currentLocation.pathname !== nextLocation.pathname,
  );

  // Confirm before discarding an in-progress import on an in-app navigation.
  useEffect(() => {
    if (blocker.state !== "blocked") return;
    if (window.confirm("Discard this import? Unsaved features will be lost.")) {
      blocker.proceed();
    } else {
      blocker.reset();
    }
    // Re-run only on a state transition — not on every blocker identity change —
    // so answering the confirm once does not re-prompt while still "blocked".
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [blocker.state]);

  return (
    <ResourceContextProvider value="geofence">
      <FormProvider {...methods}>
        <div className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
          <h1 className="text-xl font-semibold">Import</h1>
          <Stepper steps={[...IMPORT_STEPS]} active={active} />
          <div className="min-h-[40vh]">
            {active === 0 && <SourceStep onLoaded={next} />}
            {active === 1 && <MapNameStep />}
            {active === 2 && <AssignStep />}
            {active === 3 && (
              <ReviewStep onCommitted={() => setCommitted(true)} />
            )}
          </div>
          {active < IMPORT_STEPS.length - 1 && (
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
          )}
        </div>
      </FormProvider>
    </ResourceContextProvider>
  );
}

export { ImportWizard };
export default ImportWizard;
