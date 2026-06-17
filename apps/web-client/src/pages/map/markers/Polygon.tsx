/* eslint-disable @typescript-eslint/no-explicit-any */
import type { Polygon as PolygonType, MultiPolygon } from 'geojson'
import * as React from 'react'
import { Polygon } from 'react-leaflet'
import * as L from 'leaflet'

import { VECTOR_COLORS } from '@assets/constants'
import { Feature, DbOption } from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { getPolygonColor } from '@services/utils'

import { MemoPolyPopup } from '../popups/Polygon'
import Popup from '../popups/Styled'

export function KojiPolygon({
  feature,
  dbRef,
}: {
  feature: Feature<PolygonType> | Feature<MultiPolygon>
  dbRef: DbOption | null
}) {
  const { setStatic } = useStatic.getState()

  const color = getPolygonColor(`${feature.id}`)

  return (
    <Polygon
      key={`${feature.id}_${feature.geometry.type}_${feature.geometry.coordinates.length}`}
      color={color}
      eventHandlers={{
        click({ latlng }) {
          const { lat, lng } = latlng
          setStatic('clickedLocation', [lng, lat])
        },
        mouseover({ target }) {
          if (
            !useStatic.getState().combinePolyMode &&
            !Object.values(useStatic.getState().layerEditing).some((v) => v)
          ) {
            target.setStyle({ color: VECTOR_COLORS.RED })
          }
        },
        mouseout({ target }) {
          if (!useStatic.getState().combinePolyMode) {
            target.setStyle({ color })
          }
        },
        mousedown({ target }) {
          if (useStatic.getState().combinePolyMode) {
            useShapes.setState((prev) => {
              if (prev.combined[feature.id || '']) {
                target.setStyle({ color })
              } else {
                target.setStyle({ color: VECTOR_COLORS.ORANGE })
              }
              return {
                combined: {
                  ...prev.combined,
                  [feature.id || '']: !prev.combined[feature.id || ''],
                },
              }
            })
          }
        },
      }}
      ref={(ref) => {
        if (ref && feature.id) {
          ref.feature = feature
          const { type } = feature.geometry
          if (!ref.hasEventListeners('pm:remove')) {
            ref.on('pm:remove', function remove() {
              useShapes.getState().setters.remove(type, feature.id)
            })
          }
          if (!ref.hasEventListeners('pm:dragend')) {
            ref.on('pm:dragend', function dragend({ layer }) {
              if (layer instanceof L.Polygon && layer.feature && feature.id) {
                useShapes.getState().setters.update(type, feature.id, {
                  ...layer.toGeoJSON(),
                  id: feature.id,
                  properties: feature.properties,
                } as any) // TODO: fix this
              }
            })
          }
          if (!ref.hasEventListeners('pm:cut')) {
            ref.on('pm:cut', function cut({ layer, originalLayer }) {
              if (layer instanceof L.Polygon && layer.feature && feature.id) {
                useShapes.getState().setters.update(type, feature.id, {
                  ...layer.toGeoJSON(),
                  id: feature.id,
                  properties: {
                    ...feature.properties,
                    __leafletId: layer._leaflet_id,
                  },
                } as any) // TODO: fix this
                originalLayer.remove()
                layer.remove()
              }
            })
          }
          if (!ref.hasEventListeners('pm:rotateend')) {
            ref.on('pm:rotateend', function rotateEnd({ layer }) {
              if (layer instanceof L.Polygon && layer.feature && feature.id) {
                useShapes.getState().setters.update(type, feature.id, {
                  ...layer.toGeoJSON(),
                  id: feature.id,
                  properties: feature.properties,
                } as any) // TODO: fix this
              }
            })
          }
          if (!ref.hasEventListeners('pm:edit')) {
            ref.on('pm:edit', function edit({ layer }) {
              if (
                useStatic.getState().layerEditing.editMode &&
                layer instanceof L.Polygon &&
                layer.feature &&
                feature.id
              ) {
                useShapes.getState().setters.update(type, feature.id, {
                  ...layer.toGeoJSON(),
                  id: feature.id,
                  properties: feature.properties,
                } as any) // TODO: fix this
              }
            })
          }
        }
      }}
      positions={
        feature.geometry.type === 'MultiPolygon'
          ? (feature.geometry.coordinates.map((each) =>
              each.map((each_1) => each_1.map(([lng, lat]) => [lat, lng])),
            ) as [[[[number, number]]]])
          : (feature.geometry.coordinates.map((each) =>
              each.map(([lng, lat]) => [lat, lng]),
            ) as [[[number, number]]])
      }
      {...feature.properties}
      pane="polygons"
    >
      <Popup>
        <MemoPolyPopup feature={feature} dbRef={dbRef} />
      </Popup>
    </Polygon>
  )
}

export const MemoPolygon = React.memo(
  KojiPolygon,
  (prev, next) =>
    prev.feature.geometry.type === next.feature.geometry.type &&
    prev.feature.geometry.coordinates.length ===
      next.feature.geometry.coordinates.length,
)
