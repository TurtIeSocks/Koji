/* eslint-disable import/prefer-default-export */
import { Map } from 'leaflet'
import { MarkersPropsPixiOverlay } from 'react-leaflet-pixi-overlay'

import { Data } from '@assets/types'

export function getMapBounds(map: Map) {
  const mapBounds = map.getBounds()
  const { lat: minLat, lng: minLon } = mapBounds.getSouthWest()
  const { lat: maxLat, lng: maxLon } = mapBounds.getNorthEast()
  return { bounds: { minLat, maxLat, minLon, maxLon } }
}

export function buildMarkers(data: Data): MarkersPropsPixiOverlay {
  const markers: MarkersPropsPixiOverlay = []

  const { length: pointLength } = data.spawnpoints
  for (let i = 0; i < pointLength; i += 1) {
    const point = data.spawnpoints[i]
    markers.push({
      id: point.id as unknown as string, // gets converted to string by api first
      iconId: `spawnpoint_${point.despawn_sec ? 'confirmed' : 'unconfirmed'}`,
      position: [point.lat, point.lon],
      customIcon: `
        <svg xmlns="http://www.w3.org/2000/svg" fill="${
          point.despawn_sec ? 'deeppink' : 'dodgerblue'
        }" width="10" height="10" viewBox="0 0 24 24">
          <circle cx="10" cy="10" r="10" />
        </svg>
      `,
    })
  }
  const { length: gymLength } = data.gyms
  for (let i = 0; i < gymLength; i += 1) {
    const gym = data.gyms[i]
    markers.push({
      id: gym.id,
      iconId: 'gym',
      position: [data.gyms[i].lat, data.gyms[i].lon],
      customIcon: `
        <svg xmlns="http://www.w3.org/2000/svg" fill="maroon" width="20" height="20" viewBox="0 0 24 24">
          <circle cx="10" cy="10" r="10" />
        </svg>
      `,
    })
  }
  for (let i = 0; i < data.pokestops.length; i += 1) {
    markers.push({
      id: data.pokestops[i].id,
      iconId: 'pokestop',
      position: [data.pokestops[i].lat, data.pokestops[i].lon],
      customIcon: `
        <svg xmlns="http://www.w3.org/2000/svg" fill="green" width="15" height="15" viewBox="0 0 24 24">
          <circle cx="10" cy="10" r="10" />
        </svg>
      `,
    })
  }
  return markers
}

export async function getData() {
  const [pokestops, gyms, spawnpoints] = await Promise.all([
    fetch('/pokestops'),
    fetch('/gyms'),
    fetch('/spawnpoints'),
  ])
  const [pokestopsData, gymsData, spawnpointsData] = await Promise.all([
    pokestops.json(),
    gyms.json(),
    spawnpoints.json(),
  ])
  return { pokestops: pokestopsData, gyms: gymsData, spawnpoints: spawnpointsData }
}