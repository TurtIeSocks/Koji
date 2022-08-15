import { useMap } from 'react-leaflet'
import distance from '@turf/distance'

export function getMapBounds() {
  const mapBounds = useMap().getBounds()
  const { lat: min_lat, lng: min_lon } = mapBounds.getSouthWest()
  const { lat: max_lat, lng: max_lon } = mapBounds.getNorthEast()
  return { min_lat, max_lat, min_lon, max_lon }
}

export function getColor(start: [number, number], end: [number, number]) {
  const dis = distance(start, end, { units: 'meters' })
  switch (true) {
    case dis <= 500:
      return 'green'
    case dis <= 1000:
      return 'yellow'
    case dis <= 1500:
      return 'orange'
    default:
      return 'red'
  }
}
