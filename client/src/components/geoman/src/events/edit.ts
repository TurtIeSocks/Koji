import * as L from 'leaflet'

import type { HandlersWithFallback, Method } from '../types'

export default function editEvents(
  map: L.Map,
  handlers: HandlersWithFallback,
  method: Method,
) {
  map[method](
    'pm:globaleditmodetoggled',
    handlers.onGlobalEditModeToggled ?? handlers.fallback,
  )
}
