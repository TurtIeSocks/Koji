// mulberry32: a small, fast, deterministic 32-bit PRNG. Used everywhere in
// the demo world's generation code instead of Math.random() so the exact
// same seed always reproduces the exact same output stream (see
// prng.test.ts) — load-bearing for markers.ts's "generateMarkers(fc) called
// twice yields identical results" guarantee.

/** Returns a `() => number` generator yielding deterministic pseudo-random
 *  floats in `[0, 1)`, seeded by `seed`. Two generators built from the same
 *  `seed` produce identical output, draw for draw. */
export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
