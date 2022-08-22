import * as L from 'leaflet'

import type { HandlersWithFallback, Method } from '../types'

export default function dragEvents(
  map: L.Map,
  handlers: HandlersWithFallback,
  method: Method,
) {
  map[method](
    'pm:globaldragmodetoggled',
    handlers.onGlobalDragModeToggled ?? handlers.fallback,
  )
}
