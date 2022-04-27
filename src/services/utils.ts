/* eslint-disable import/prefer-default-export */
import { Map } from 'leaflet'

export function getMapBounds(map: Map) {
  const mapBounds = map.getBounds()
  const { lat: minLat, lng: minLon } = mapBounds.getSouthWest()
  const { lat: maxLat, lng: maxLon } = mapBounds.getNorthEast()
  return { bounds: { minLat, maxLat, minLon, maxLon } }
}
