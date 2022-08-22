import * as L from 'leaflet'

import type { HandlersWithFallback, Method } from '../types'

export default function cutEvents(
  map: L.Map,
  handlers: HandlersWithFallback,
  method: Method,
) {
  map[method](
    'pm:globalcutmodetoggled',
    handlers.onGlobalCutModeToggled ?? handlers.fallback,
  )
  map[method](
    'pm:globalcutmodetoggled',
    handlers.onGlobalCutModeToggled ?? handlers.fallback,
  )
  map.on('pm:cut', handlers.onMapCut ?? handlers.fallback)
}
