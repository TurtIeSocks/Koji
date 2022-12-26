/* eslint-disable @typescript-eslint/no-explicit-any */
import type { Feature, Polygon as PolygonType, MultiPolygon } from 'geojson'
import * as React from 'react'
import { Polygon } from 'react-leaflet'

import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import * as L from 'leaflet'
import PolygonPopup from '@components/popups/Polygon'
import StyledPopup from '@components/popups/Styled'

export default function KojiPolygon({
  feature,
}: {
  feature: Feature<PolygonType> | Feature<MultiPolygon>
}) {
  const [loadData, setLoadData] = React.useState(false)

  return (
    <Polygon
      key={feature.id}
      ref={(ref) => {
        if (ref && feature.id) {
          ref.feature = feature
          const { type } = feature.geometry
          ref.on('click', () => setLoadData(true))
          ref.removeEventListener('pm:remove')
          ref.on('pm:remove', function remove() {
            useShapes.getState().setters.remove(type, feature.id)
          })
          ref.removeEventListener('pm:dragend')
          ref.on('pm:dragend', function dragend({ layer }) {
            if (layer instanceof L.Polygon && layer.feature && feature.id) {
              useShapes.getState().setters.update(type, feature.id, {
                ...layer.toGeoJSON(),
                id: feature.id,
                properties: feature.properties,
              } as any) // TODO: fix this
            }
          })
          ref.removeEventListener('pm:cut')
          ref.on('pm:cut', function cut({ layer, originalLayer }) {
            if (layer instanceof L.Polygon && layer.feature && feature.id) {
              useShapes.getState().setters.update(type, feature.id, {
                ...layer.toGeoJSON(),
                id: feature.id,
                properties: {
                  ...feature.properties,
                  leafletId: layer._leaflet_id,
                },
              } as any) // TODO: fix this
              originalLayer.remove()
              layer.remove()
            }
          })
          ref.removeEventListener('pm:rotateend')
          ref.on('pm:rotateend', function rotateEnd({ layer }) {
            if (layer instanceof L.Polygon && layer.feature && feature.id) {
              useShapes.getState().setters.update(type, feature.id, {
                ...layer.toGeoJSON(),
                id: feature.id,
                properties: feature.properties,
              } as any) // TODO: fix this
            }
          })
          ref.removeEventListener('pm:edit')
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
      pmIgnore={false}
      pane="polygons"
    >
      <StyledPopup>
        <PolygonPopup feature={feature} loadData={loadData} />
      </StyledPopup>
    </Polygon>
  )
}
