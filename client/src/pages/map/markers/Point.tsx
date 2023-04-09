import * as React from 'react'
import type { Point } from 'geojson'
import { Circle, Tooltip } from 'react-leaflet'
import * as L from 'leaflet'
import geohash from 'ngeohash'

import { VECTOR_COLORS } from '@assets/constants'
import type { Feature, DbOption, KojiKey } from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { s2Coverage } from '@services/fetches'
import { getPointColor } from '@services/utils'

import BasePopup from '../popups/Styled'
import { MemoPointPopup } from '../popups/Point'

export function KojiPoint({
  feature,
  radius,
  type = 'Point',
  dbRef,
  index,
  combined,
}: {
  feature: Feature<Point>
  radius: number
  type?: 'Point' | 'MultiPoint'
  dbRef: DbOption | null
  parentId?: KojiKey
  index?: number
  combined?: boolean
}) {
  const {
    id,
    properties,
    geometry: {
      coordinates: [lon, lat],
    },
  } = feature
  const color = combined
    ? VECTOR_COLORS.ORANGE
    : getPointColor(`${properties?.__multipoint_id || id}`, type, index || 0)

  return (
    <Circle
      key={`${combined}`}
      eventHandlers={{
        mouseover() {
          if (
            type === 'MultiPoint' &&
            properties?.__multipoint_id &&
            usePersist.getState().setActiveMode === 'hover' &&
            !useStatic.getState().combinePolyMode
          ) {
            useShapes
              .getState()
              .setters.activeRoute(properties?.__multipoint_id)
          }
        },
        click({ target }) {
          if (useStatic.getState().combinePolyMode) {
            target.closePopup()
            useShapes.setState((prev) => {
              const pointId =
                (type === 'MultiPoint' ? properties?.__multipoint_id : id) || ''
              if (prev.combined[pointId]) {
                target.setStyle({ color })
              } else {
                target.setStyle({ color: VECTOR_COLORS.ORANGE })
              }
              return {
                combined: {
                  ...prev.combined,
                  [pointId]: !prev.combined[pointId],
                },
              }
            })
          } else if (
            type === 'MultiPoint' &&
            properties?.__multipoint_id &&
            usePersist.getState().setActiveMode === 'click'
          ) {
            useShapes
              .getState()
              .setters.activeRoute(properties?.__multipoint_id)
          }
        },
      }}
      ref={(circle) => {
        if (circle && id !== undefined) {
          circle.feature = feature
          if (circle.feature?.properties && (index || index === 0)) {
            circle.feature.properties.__index = index
          }
          // if (circle.isPopupOpen()) {
          //   circle.closePopup()
          // }
          circle.removeEventListener('pm:remove')
          circle.on('pm:remove', function remove() {
            useShapes.getState().setters.remove(type, id)
          })
          circle.removeEventListener('pm:drag')
          circle.on('pm:drag', async function drag({ layer }) {
            if (layer instanceof L.Circle) {
              const latlng = layer.getLatLng()
              const coverage = await s2Coverage(
                `${properties.__multipoint_id}__${id}`,
                latlng.lat,
                latlng.lng,
              )
              useShapes.setState({ s2cellCoverage: coverage })
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
              const coverage = await s2Coverage(
                `${properties.__multipoint_id}__${id}`,
                newLat,
                newLon,
              )
              useShapes.setState({ s2cellCoverage: coverage })
            }
          })
        }
      }}
      radius={radius}
      center={[lat, lon]}
      snapIgnore
      pmIgnore={type === 'MultiPoint'}
      pane="circles"
      {...properties}
      color={color}
    >
      {!useStatic.getState().combinePolyMode && (
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
      )}
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
    prev.combined === next.combined &&
    prev.feature.geometry.coordinates.every(
      (coord, i) => next.feature.geometry.coordinates[i] === coord,
    ),
)
