// Opens a geofence's edit page in a new tab, under the hash router. Shared by
// every read-only overlay that renders OTHER geofences (project member maps,
// geofence ghost neighbors) so clicking one jumps straight to its editor.
export function openGeofenceEdit(id: number | string): void {
  const url = `${window.location.origin}${window.location.pathname}#/geofence/${id}`;
  window.open(url, "_blank", "noopener");
}
