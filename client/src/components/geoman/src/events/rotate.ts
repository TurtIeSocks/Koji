import * as L from 'leaflet'

import type { HandlersWithFallback, Method } from '../types'

export default function rotateEvents(
  map: L.Map,
  handlers: HandlersWithFallback,
  method: Method,
) {
  map[method](
    'pm:globalrotatemodetoggled',
    handlers.onGlobalRotateModeToggled ?? handlers.fallback,
  )
  map.on('pm:rotateenable', handlers.onMapRotateEnable ?? handlers.fallback)
  map.on('pm:rotatedisable', handlers.onMapRotateDisable ?? handlers.fallback)
  map.on('pm:rotate', handlers.onMapRotate ?? handlers.fallback)
  map.on('pm:rotatestart', handlers.onMapRotateStart ?? handlers.fallback)
  map.on('pm:rotateend', handlers.onMapRotateEnd ?? handlers.fallback)
}
