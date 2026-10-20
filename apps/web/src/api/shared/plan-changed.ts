import type { PublishPlan } from "../types";

/** A send whose approved writes no longer match the server's plan (409): nothing
 *  was sent, and `plan` is what a send would write now. */
export class PlanChangedError extends Error {
  constructor(public readonly plan: PublishPlan) {
    super("The publish plan changed; nothing was sent");
    this.name = "PlanChangedError";
  }
}
