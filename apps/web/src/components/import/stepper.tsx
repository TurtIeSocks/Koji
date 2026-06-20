import { cn } from "@/lib/utils";

interface StepperProps {
  steps: string[];
  active: number;
}

/** Horizontal step indicator for the import wizard. Presentational only —
 *  the wizard owns the `active` index. */
function Stepper({ steps, active }: StepperProps) {
  return (
    <ol className="flex w-full items-center gap-2" role="list">
      {steps.map((label, i) => {
        const state =
          i < active ? "complete" : i === active ? "active" : "upcoming";
        return (
          <li
            key={label}
            aria-current={state === "active" ? "step" : undefined}
            className="flex flex-1 items-center gap-2"
          >
            <span
              className={cn(
                "flex h-7 w-7 shrink-0 items-center justify-center rounded-full border text-sm font-medium",
                state === "active" &&
                  "border-primary bg-primary text-primary-foreground",
                state === "complete" && "border-primary bg-primary/10 text-primary",
                state === "upcoming" && "border-muted text-muted-foreground",
              )}
            >
              {i + 1}
            </span>
            <span
              className={cn(
                "truncate text-sm",
                state === "upcoming"
                  ? "text-muted-foreground"
                  : "text-foreground",
              )}
            >
              {label}
            </span>
            {i < steps.length - 1 && <span className="h-px flex-1 bg-border" />}
          </li>
        );
      })}
    </ol>
  );
}

export { Stepper, type StepperProps };
