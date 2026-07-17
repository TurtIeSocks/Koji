/* eslint-disable @typescript-eslint/no-explicit-any */
// Pure data-shape transform — no network dependency, so both index.live.ts
// and index.demo.ts re-export the same implementation (unlike the rest of
// `@api`, which forks per mode). Lives outside live/ and demo/ so neither
// mode "owns" it.

/** Strip a geofence's `properties` rows to the write wire shape
 *  `{ property_id, value }`, dropping read-only keys (id/name/category/...).
 *  Idempotent; returns data unchanged when there are no properties. */
export const serializeGeofenceWrite = (data: any): any => {
  if (!Array.isArray(data?.properties)) return data;
  return {
    ...data,
    properties: data.properties.map((p: any) => ({
      property_id: p.property_id,
      value: p.value,
    })),
  };
};
