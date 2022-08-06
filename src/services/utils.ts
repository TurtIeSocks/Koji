import type {
  FeatureGroup,
  LatLng,
  LayerGroup,
  Polygon,
  CircleMarker,
} from 'leaflet'
import L from 'leaflet'
import { useMap } from 'react-leaflet'
import type { LineString, Feature } from '@turf/helpers'
import distance from '@turf/distance'
import polygonToLine from '@turf/polygon-to-line'
import pointToLineDistance from '@turf/point-to-line-distance'
import inside from '@turf/boolean-point-in-polygon'

export function getMapBounds() {
  const mapBounds = useMap().getBounds()
  const { lat: min_lat, lng: min_lon } = mapBounds.getSouthWest()
  const { lat: max_lat, lng: max_lon } = mapBounds.getNorthEast()
  return { min_lat, max_lat, min_lon, max_lon }
}

export function getButtonHtml(hash: string, icon: string) {
  return `
<a href="#${hash}" style="display: flex; align-items: center; justify-content: center;">
  <span class="material-icons">${icon}</span>
</a>`
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

export function destination(latlng: LatLng, heading: number, dis: number) {
  const rad = Math.PI / 180
  const radInv = 180 / Math.PI

  const newHeading = (heading + 360) % 360
  const rheading = newHeading * rad
  const lon1 = latlng.lng * rad
  const lat1 = latlng.lat * rad
  const sinLat1 = Math.sin(lat1)
  const cosLat1 = Math.cos(lat1)

  const R = 6378137
  const cosDistR = Math.cos(dis / R)
  const sinDistR = Math.sin(dis / R)
  const lat2 = Math.asin(
    sinLat1 * cosDistR + cosLat1 * sinDistR * Math.cos(rheading),
  )
  let lon2 =
    lon1 +
    Math.atan2(
      Math.sin(rheading) * sinDistR * cosLat1,
      cosDistR - sinLat1 * Math.sin(lat2),
    )
  lon2 *= radInv
  // eslint-disable-next-line no-nested-ternary
  lon2 = lon2 > 180 ? lon2 - 360 : lon2 < -180 ? lon2 + 360 : lon2
  return L.latLng([lat2 * radInv, lon2])
}

export function genCircles(polygons: FeatureGroup, circles: FeatureGroup) {
  const xMod = Math.sqrt(0.75)
  const yMod = Math.sqrt(0.568)
  const settings = { circleSize: 70 }

  circles.clearLayers()
  const route = (layer: Polygon) => {
    const poly = layer.toGeoJSON()
    const line = polygonToLine(poly) as Feature<LineString>

    let currentLatLng = layer.getBounds().getNorthEast()

    const startLatLng = destination(currentLatLng, 90, 80 * 1.5)
    const endLatLng = destination(
      destination(
        layer.getBounds().getSouthWest(),
        270,
        settings.circleSize * 1.5,
      ),
      180,
      settings.circleSize,
    )
    let row = 0
    let heading = 270
    let i = 0
    const routes: [number, number][] = []
    while (currentLatLng.lat > endLatLng.lat) {
      while (
        (heading === 270 && currentLatLng.lng > endLatLng.lng) ||
        (heading === 90 && currentLatLng.lng < startLatLng.lng)
      ) {
        const point: {
          type: 'Feature'
          geometry: {
            type: 'Point'
            coordinates: [number, number]
          }
          properties: { [key: string]: string | number }
        } = {
          type: 'Feature',
          geometry: {
            type: 'Point',
            coordinates: [currentLatLng.lng, currentLatLng.lat],
          },
          properties: {},
        }
        const pointDistance = pointToLineDistance(point, line, {
          units: 'meters',
        })
        if (
          pointDistance <= settings.circleSize ||
          pointDistance === 0 ||
          inside(point, poly)
        ) {
          routes.push([currentLatLng.lat, currentLatLng.lng])
          L.circle(currentLatLng, {
            color: 'red',
            fillColor: '#f03',
            fillOpacity: 0.5,
            radius: settings.circleSize,
          })
            .bindPopup(
              `<button class="btn btn-secondary btn-sm deleteLayer" data-layer-container="circleLayer" data-layer-id=${i} type="button">Delete</button></div>`,
            )
            .addTo(circles)
        }
        currentLatLng = destination(
          currentLatLng,
          heading,
          xMod * settings.circleSize * 2,
        )
      }

      currentLatLng = destination(
        currentLatLng,
        180,
        yMod * settings.circleSize * 2,
      )
      if (row % 2 === 1) {
        heading = 270
      } else {
        heading = 90
      }
      currentLatLng = destination(
        currentLatLng,
        heading,
        xMod * settings.circleSize * 3,
      )
      row += 1
    }
    i += 1
  }

  polygons.eachLayer((layer) => {
    route(layer as Polygon)
  })
}

export function degreesToRadians(degrees: number) {
  return (degrees * Math.PI) / 180
}

export function distanceInKm(
  lat1: number,
  lon1: number,
  lat2: number,
  lon2: number,
) {
  const earthRadiusKm = 6371

  const dLat = degreesToRadians(lat2 - lat1)
  const dLon = degreesToRadians(lon2 - lon1)

  const rLat1 = degreesToRadians(lat1)
  const rLat2 = degreesToRadians(lat2)

  const a =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.sin(dLon / 2) * Math.sin(dLon / 2) * Math.cos(rLat1) * Math.cos(rLat2)
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a))
  return earthRadiusKm * c
}

export function getRoute(routingLayer: FeatureGroup, circleLayer: LayerGroup) {
  const settings = { greenRouteAmount: 500, redRouteAmount: 1000 }

  // Draw the Lines and numbers
  routingLayer.clearLayers()
  const allCircles = circleLayer.getLayers() as CircleMarker[]
  for (let i = 0; i < allCircles.length; i += 1) {
    const circleLat = allCircles[i].getLatLng().lat
    const circleLng = allCircles[i].getLatLng().lng
    let nextCircleLat = 0
    let nextCircleLng = 0
    if (i !== allCircles.length - 1) {
      nextCircleLat = allCircles[i + 1].getLatLng().lat
      nextCircleLng = allCircles[i + 1].getLatLng().lng
    } else if (i === allCircles.length - 1) {
      nextCircleLat = allCircles[0].getLatLng().lat
      nextCircleLng = allCircles[0].getLatLng().lng
    }
    const currentDistance =
      Math.round(
        distanceInKm(circleLat, circleLng, nextCircleLat, nextCircleLng) *
          1000 *
          100,
      ) / 100
    let color = ''
    if (currentDistance < settings.greenRouteAmount) {
      color = 'green'
    } else if (
      currentDistance > settings.greenRouteAmount &&
      currentDistance < settings.redRouteAmount
    ) {
      color = 'orange'
    } else {
      color = 'red'
    }
    const routing = L.polyline(
      [
        [circleLat, circleLng],
        [nextCircleLat, nextCircleLng],
      ],
      { color },
    )
    routing.addTo(routingLayer)
  }
}
