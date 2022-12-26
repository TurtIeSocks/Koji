/* eslint-disable prefer-arrow-callback */
import * as React from 'react'
import type { Feature, Point } from 'geojson'
import { Circle } from 'react-leaflet'
import { useShapes } from '@hooks/useShapes'
import * as L from 'leaflet'
import StyledPopup from '@components/popups/Styled'
import PointPopup from '@components/popups/Point'

export default function KojiPoint({
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
                type: 'Feature',
                id,
                properties,
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
      <StyledPopup>
        <PointPopup id={id} properties={properties} lat={lat} lon={lon} />
      </StyledPopup>
    </Circle>
  )
}
