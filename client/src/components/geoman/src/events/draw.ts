import * as L from 'leaflet'

import layerEvents from './layers'
import type { HandlersWithFallback, Method } from '../types'

export default function drawEvents(
  map: L.Map,
  handlers: HandlersWithFallback,
  method: Method,
) {
  map[method](
    'pm:globaldrawmodetoggled',
    handlers.onGlobalDrawModeToggle ?? handlers.fallback,
  )
  map[method]('pm:create', (e) => {
    layerEvents(e.layer, handlers, method)
    if (handlers.onCreate) {
      handlers.onCreate(e)
    } else {
      handlers.fallback(e)
    }
  })
  map[method]('pm:drawstart', (e) => {
    layerEvents(e.workingLayer, handlers, method)
    if (handlers.onDrawStart) {
      handlers.onDrawStart(e)
    } else {
      handlers.fallback(e)
    }
  })
  map[method]('pm:drawend', handlers.onDrawEnd ?? handlers.fallback)
}
