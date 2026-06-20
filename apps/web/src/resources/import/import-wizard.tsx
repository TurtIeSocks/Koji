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

  // --- Leave-guard: in-app navigation (react-router v8 data router) -----
  // ponytail: useBlocker only works when the router is a data router (TanStack/
  // RR v6.4+ createBrowserRouter). If the Admin shell uses the legacy BrowserRouter,
  // this throws on mount — wrap in try/catch and fall back to beforeunload-only.
  let blocker: ReturnType<typeof useBlocker> | null = null;
  try {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    blocker = useBlocker(
      ({ currentLocation, nextLocation }) =>
        isDirty && !committed && currentLocation.pathname !== nextLocation.pathname,
    );
  } catch {
    // ponytail: Admin router is not a data router — in-app blocker not available.
    // beforeunload guard above still covers tab-close/reload.
    blocker = null;
  }

  // Show confirm dialog when in-app blocker fires
  useEffect(() => {
    if (!blocker || blocker.state !== "blocked") return;
    if (
      window.confirm(
        "Discard this import? Unsaved features will be lost.",
      )
    ) {
      blocker.proceed();
    } else {
      blocker.reset();
    }
  }, [blocker, blocker?.state]);

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
