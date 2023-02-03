import * as React from 'react'
import type { Point } from 'geojson'
import { Circle } from 'react-leaflet'
import * as L from 'leaflet'

import type { Feature, DbOption, KojiKey } from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'

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
  dbRef,
}: {
  feature: Feature<Point>
  radius: number
  type?: 'Point' | 'MultiPoint'
  dbRef: DbOption | null
  parentId?: KojiKey
}) {
  return (
    <Circle
      ref={(circle) => {
        if (circle && id !== undefined) {
          if (circle.isPopupOpen()) circle.closePopup()
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
          if (usePersist.getState().setActiveMode === 'hover') {
            circle.removeEventListener('mouseover')
            circle.on('mouseover', function onClick() {
              if (
                type === 'MultiPoint' &&
                properties?.__multipoint_id &&
                !Object.values(useStatic.getState().layerEditing).some((v) => v)
              ) {
                useShapes
                  .getState()
                  .setters.activeRoute(properties?.__multipoint_id)
              }
            })
          } else {
            circle.on('click', function onClick() {
              if (type === 'MultiPoint' && properties?.__multipoint_id) {
                useShapes
                  .getState()
                  .setters.activeRoute(properties?.__multipoint_id)
              }
            })
          }
        }
      }}
      radius={radius}
      center={[lat, lon]}
      snapIgnore
      pmIgnore={type === 'MultiPoint'}
      pane="circles"
      {...properties}
      color={type === 'Point' ? '#3388ff' : 'red'}
    >
      <BasePopup>
        <MemoPointPopup
          id={id}
          properties={properties}
          lat={lat}
          lon={lon}
          type={type}
          dbRef={dbRef}
        />
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
