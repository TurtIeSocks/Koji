import * as React from 'react'
import type { Point } from 'geojson'
import { Circle, Tooltip } from 'react-leaflet'
import * as L from 'leaflet'
import geohash from 'ngeohash'

import type { Feature, DbOption, KojiKey } from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { s2Coverage } from '@services/fetches'

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
  index,
}: {
  feature: Feature<Point>
  radius: number
  type?: 'Point' | 'MultiPoint'
  dbRef: DbOption | null
  parentId?: KojiKey
  index?: number
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
          circle.removeEventListener('pm:drag')
          circle.on('pm:drag', async function drag({ layer }) {
            if (layer instanceof L.Circle) {
              const latlng = layer.getLatLng()
              useShapes.setState({
                s2cellCoverage: await s2Coverage(id, latlng.lat, latlng.lng),
              })
            }
          })
          circle.removeEventListener('pm:dragend')
          circle.on('pm:dragend', async function dragend({ layer }) {
            if (layer instanceof L.Circle) {
              const { lat: newLat, lng: newLon } = circle.getLatLng()
              useShapes.getState().setters.update(type, id, {
                ...useShapes.getState().Point[id],
                geometry: { type: 'Point', coordinates: [newLon, newLat] },
              })
              useShapes.setState({
                s2cellCoverage: await s2Coverage(id, newLat, newLon),
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
      color={
        type === 'Point'
          ? 'red'
          : properties?.__multipoint_id?.toString().includes('__SCANNER')
          ? 'green'
          : '#3388ff'
      }
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
      {!!index && (
        <Tooltip direction="top">
          {index} -
          {process.env.NODE_ENV === 'development' &&
            geohash.encode(lat, lon, 9)}
        </Tooltip>
      )}
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
