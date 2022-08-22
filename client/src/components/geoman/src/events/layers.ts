import * as L from 'leaflet'
import type { HandlersWithFallback, Method } from '../types'

export default function layerEvents(
  layer: L.Layer,
  handlers: HandlersWithFallback,
  status: Method,
) {
  // Drawing Mode
  layer[status]('pm:vertexadded', handlers.onVertexAdded ?? handlers.fallback)
  layer[status]('pm:snapdrag', handlers.onSnapDrag ?? handlers.fallback)
  layer[status]('pm:snap', handlers.onSnap ?? handlers.fallback)
  layer[status]('pm:unsnap', handlers.onUnsnap ?? handlers.fallback)
  layer[status]('pm:centerplaced', handlers.onCenterPlaced ?? handlers.fallback)
  layer[status]('pm:change', handlers.onChange ?? handlers.fallback)

  // Edit Mode
  layer[status]('pm:edit', handlers.onEdit ?? handlers.fallback)
  layer[status]('pm:update', (e) => {
    if (handlers.onUpdate) {
      handlers.onUpdate(e)
    } else {
      handlers.fallback()
    }
  })
  layer[status]('pm:enable', handlers.onEnable ?? handlers.fallback)
  layer[status]('pm:disable', handlers.onDisable ?? handlers.fallback)
  layer[status](
    'pm:vertexremoved',
    handlers.onVertexRemoved ?? handlers.fallback,
  )
  layer[status]('pm:vertexclick', handlers.onVertexClick ?? handlers.fallback)
  layer[status](
    'pm:markerdragstart',
    handlers.onMarkerDragStart ?? handlers.fallback,
  )
  layer[status]('pm:markerdrag', handlers.onMarkerDrag ?? handlers.fallback)
  layer[status](
    'pm:markerdragend',
    handlers.onMarkerDragEnd ?? handlers.fallback,
  )
  layer[status]('pm:layerreset', handlers.onLayerReset ?? handlers.fallback)
  layer[status]('pm:intersect', handlers.onIntersect ?? handlers.fallback)

  // Drag Mode
  layer[status]('pm:dragstart', handlers.onDragStart ?? handlers.fallback)
  layer[status]('pm:drag', handlers.onDrag ?? handlers.fallback)
  layer[status]('pm:dragend', handlers.onDragEnd ?? handlers.fallback)
  layer[status]('pm:dragenable', handlers.onDragEnable ?? handlers.fallback)
  layer[status]('pm:dragdisable', handlers.onDragDisable ?? handlers.fallback)

  // Remove Mode
  layer[status]('pm:remove', handlers.onLayerRemove ?? handlers.fallback)

  // Cut Mode
  layer[status]('pm:cut', handlers.onLayerCut ?? handlers.fallback)

  // Rotate Mode
  layer[status](
    'pm:rotateenable',
    handlers.onLayerRotateEnable ?? handlers.fallback,
  )
  layer[status](
    'pm:rotatedisable',
    handlers.onLayerRotateDisable ?? handlers.fallback,
  )
  layer[status](
    'pm:rotatestart',
    handlers.onLayerRotateStart ?? handlers.fallback,
  )
  layer[status]('pm:rotate', handlers.onLayerRotate ?? handlers.fallback)
  layer[status]('pm:rotateend', handlers.onLayerRotateEnd ?? handlers.fallback)

  // Text Mode
  layer[status]('pm:textchange', handlers.onTextChange ?? handlers.fallback)
}
