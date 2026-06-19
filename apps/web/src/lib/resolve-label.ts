/**
 * Returns a display label honoring both i18n keys and literal user-supplied
 * strings.
 *
 * `ra-core`'s `translate()` is built on Polyglot, which returns the `_`
 * fallback whenever the first argument is not a registered i18n key. The
 * common pattern `translate(label, { _: "Refresh" })` therefore SILENTLY
 * REPLACES any user-supplied literal label (e.g. `label="Reload data"`) with
 * the fallback string, because the literal is not a registered key.
 *
 * Convention used here: i18n keys are dot-namespaced (`ra.action.refresh`,
 * `myapp.users.invite`). Any other string is treated as a literal label and
 * returned as-is.
 *
 * @param label    The label prop, or undefined.
 * @param translate The `translate` function returned by `useTranslate()`.
 * @param fallback A default label used when `label` is undefined.
 *
 * @example
 * const translatedLabel = resolveLabel(label, translate, "Refresh");
 * // label="ra.action.refresh"  → "Refresh" (or locale equivalent)
 * // label="Reload data"        → "Reload data"
 * // label=undefined            → "Refresh"
 */
export function resolveLabel(
  label: string | undefined,
  translate: (key: string, options?: { _: string }) => string,
  fallback: string,
): string {
  if (label == null || label === "") return fallback;
  // Heuristic: i18n keys are dot-namespaced. Literal labels are not.
  if (label.includes(".")) return translate(label, { _: fallback });
  return label;
}
