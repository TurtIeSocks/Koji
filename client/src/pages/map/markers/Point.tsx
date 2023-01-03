import * as React from 'react'
import type { Feature, Point } from 'geojson'
import { Circle } from 'react-leaflet'
import * as L from 'leaflet'

import { useShapes } from '@hooks/useShapes'

import BasePopup from '../popups/Styled'
import { MemoPointPopup } from '../popups/Point'

export function KojiPoint({
  feature: {
    id,
    properties,
    geometry: {
      coordinates: [lon, lat],
    },
  },
  radius,
  type = 'Point',
}: {
  feature: Feature<Point>
  radius: number
  type?: 'Point' | 'MultiPoint'
}) {
  return (
    <Circle
      ref={(circle) => {
        if (circle && id) {
          circle.removeEventListener('pm:remove')
          circle.on('pm:remove', function remove() {
            useShapes.getState().setters.remove(type, id)
          })
          circle.removeEventListener('pm:dragend')
          circle.on('pm:dragend', function dragend({ layer }) {
            if (layer instanceof L.Circle) {
              const { lat: newLat, lng: newLon } = circle.getLatLng()
              useShapes.getState().setters.update(type, id, {
                ...useShapes.getState().Point[id],
                geometry: { type: 'Point', coordinates: [newLon, newLat] },
              })
            }
          })
        }
      }}
      radius={radius}
      center={[lat, lon]}
      snapIgnore
      pane="circles"
      {...properties}
    >
      <BasePopup>
        <MemoPointPopup id={id} properties={properties} lat={lat} lon={lon} />
      </BasePopup>
    </Circle>
  )
}

export const MemoPoint = React.memo(
  KojiPoint,
  (prev, next) =>
    prev.feature.id === next.feature.id &&
    prev.radius === next.radius &&
    prev.feature.geometry.coordinates.every(
      (coord, i) => next.feature.geometry.coordinates[i] === coord,
    ),
)
