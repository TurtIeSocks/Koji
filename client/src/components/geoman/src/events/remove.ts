import * as L from 'leaflet'

import type { HandlersWithFallback, Method } from '../types'

export default function removeEvents(
  map: L.Map,
  handlers: HandlersWithFallback,
  method: Method,
) {
  map[method](
    'pm:globalremovalmodetoggled',
    handlers.onGlobalRemovalModeToggled ?? handlers.fallback,
  )
  map.on('pm:remove', handlers.onMapRemove ?? handlers.fallback)
}
