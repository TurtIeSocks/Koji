/* eslint-disable prefer-arrow-callback */
import * as React from 'react'
import type { Feature, Point } from 'geojson'
import { Circle as BaseCircle, Popup } from 'react-leaflet'
import geohash from 'ngeohash'
import { useShapes } from '@hooks/useShapes'
import * as L from 'leaflet'

export default function Circle({
  feature: {
    id,
    properties,
    geometry: {
      coordinates: [lon, lat],
    },
  },
  radius,
  type = 'points',
}: {
  feature: Feature<Point>
  radius: number
  type?: 'points' | 'multiPoints'
}) {
  return (
    <BaseCircle
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
      <Popup>
        <div>
          {JSON.stringify({ id, properties }, null, 2)}
          <br />
          Lat: {lat.toFixed(6)}
          <br />
          Lng: {lon.toFixed(6)}
          <br />
          Hash: {geohash.encode(lat, lon, 9)}
          <br />
          Hash: {geohash.encode(lat, lon, 12)}
        </div>
      </Popup>
    </BaseCircle>
  )
}
