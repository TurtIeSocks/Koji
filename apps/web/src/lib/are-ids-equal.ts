/**
 * Compares two record ids for equality, coercing to strings so
 * numeric ids sourced from APIs match URL-string ids in form state.
 */
export const areIdsEqual = (a: unknown, b: unknown): boolean => {
  if (a === b) return true;
  if (a == null || b == null) return false;
  return String(a) === String(b);
};
