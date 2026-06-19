/**
 * Consolidated `any` escape hatch.
 *
 * Use `Any` (never the bare `any` keyword) wherever a concrete type genuinely
 * cannot be expressed. Routing every deliberate opt-out of type safety through
 * one named alias keeps them greppable and keeps the lint suppression in
 * exactly one place — here.
 *
 * Current callers: the guesser field-type maps (`*-field-types.tsx`,
 * `*-guesser.tsx`). They bridge ra-core's untyped `InferredTypeMap`
 * (`component?: ComponentType`, i.e. all-optional props) to concrete input/
 * field components that require `source: string`. That contravariant mismatch
 * cannot be satisfied by any single concrete type: a shared all-optional props
 * interface fails the `{...props}` spreads, precise `ComponentProps<typeof X>`
 * fails the `ComponentType` assignment, and `as InferredTypeMap` reports
 * insufficient overlap. So `any` is load-bearing in that glue.
 *
 * To tighten later, grep for `: Any`, `<Any`, `as Any` and `Any>` and replace
 * as ra-core's inference types allow.
 */
// biome-ignore lint/suspicious/noExplicitAny: single consolidated `any` escape hatch; see the doc comment above for why concrete types are infeasible at the ra-core inference boundary.
export type Any = any;
