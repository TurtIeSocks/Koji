import type { DbOption } from '@assets/types'

export function getKojiGeofenceId(
  featureId: string | number,
  fence: DbOption | null,
  route: DbOption | null,
  geofences: Record<string | number, DbOption>,
): number | undefined {
  if (!fence) return undefined
  if (featureId.toString().endsWith('__KOJI')) return fence.id

  return (
    route?.geofence_id ??
    Object.values(geofences).find(({ name }) => name === fence.name)?.id
  )
}
