import * as L from 'leaflet'
import { capitalize } from '@mui/material'
import type { MultiPoint, MultiPolygon, Point, Polygon } from 'geojson'
import union from '@turf/union'
import bbox from '@turf/bbox'
import { useStatic } from '@hooks/useStatic'
import booleanPointInPolygon from '@turf/boolean-point-in-polygon'
import { useShapes } from '@hooks/useShapes'
import {
  Category,
  Feature,
  FeatureCollection,
  KojiModes,
  KojiRouteModes,
} from '@assets/types'
import { usePersist } from '@hooks/usePersist'
import { VECTOR_COLORS } from '@assets/constants'

export function getMapBounds(map: L.Map) {
  const mapBounds = map.getBounds()
  const { lat: min_lat, lng: min_lon } = mapBounds.getSouthWest()
  const { lat: max_lat, lng: max_lon } = mapBounds.getNorthEast()
  return { min_lat, max_lat, min_lon, max_lon }
}

export function getColor(dis: number) {
  const { lineColorRules } = usePersist.getState()
  if (lineColorRules.length) {
    for (let i = 0; i < lineColorRules.length; i++) {
      if (dis <= lineColorRules[i].distance) {
        return lineColorRules[i].color
      }
    }
  } else {
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
  return 'red'
}

export function fromCamelCase(str: string, separator = ' '): string {
  return capitalize(str)
    .replace(/([a-z\d])([A-Z])/g, `$1${separator}$2`)
    .replace(/([A-Z]+)([A-Z][a-z\d]+)/g, `$1${separator}$2`)
}

export function fromSnakeCase(str: string, separator = ' '): string {
  return capitalize(str)
    .replace(/_/g, separator)
    .replace(/([a-z\d])([A-Z])/g, `$1${separator}$2`)
    .replace(/([A-Z]+)([A-Z][a-z\d]+)/g, `$1${separator}$2`)
}

export function safeParse<T>(value: string): {
  value: T
  error: boolean | string
} {
  try {
    return { value: JSON.parse(value), error: false }
  } catch (e) {
    if (e instanceof Error) {
      return { error: e.message, value: null as T }
    }
    return { error: true, value: null as T }
  }
}

export function collectionToObject(collection: FeatureCollection) {
  return Object.fromEntries(
    collection.features.map((feat) => [
      `${feat.properties?.__name}__${feat.properties?.__mode}`,
      feat,
    ]),
  )
}

export function filterImports<T extends Feature>(
  existing: Record<string, T>,
): Record<string, T> {
  return Object.fromEntries(
    Object.values(existing)
      .filter((feat) => typeof feat.id === 'number')
      .map((feat) => [feat.id, feat]),
  )
}

export function combineByProperty(
  featureCollection: FeatureCollection,
  key = 'name',
): FeatureCollection {
  const featureHash: Record<string, Feature<Polygon | MultiPolygon>> = {}
  featureCollection.features.forEach((feat) => {
    const name = feat.properties?.[key]
    if (
      name &&
      (feat.geometry.type === 'Polygon' ||
        feat.geometry.type === 'MultiPolygon')
    ) {
      const existing = featureHash[name]
      if (existing) {
        const merged = union(existing, feat as Feature<Polygon | MultiPolygon>)
        if (merged) {
          featureHash[name] = {
            ...existing,
            ...(merged as Feature<Polygon | MultiPolygon>),
            properties: {
              ...existing.properties,
              ...feat.properties,
            },
          }
        }
      } else {
        featureHash[name] = feat as Feature<Polygon | MultiPolygon>
      }
    }
  })
  return {
    ...featureCollection,
    bbox: bbox(featureCollection),
    features: Object.values(featureHash).map((feat) => ({
      ...feat,
      bbox: bbox(feat),
    })),
  }
}

export function splitMultiPolygons(
  featureCollection: FeatureCollection,
): FeatureCollection {
  const features: Feature[] = []
  const { Polygon } = useShapes.getState()
  const userIds: Set<string> = new Set([...Object.keys(Polygon)])

  const getId = (id: number, mode: KojiModes) => {
    let newId = id
    while (userIds.has(`${newId}__${mode}__CLIENT`)) {
      newId += 1
    }
    const safeId = `${newId}__${mode}__CLIENT`
    userIds.add(safeId)
    return safeId
  }
  featureCollection.features.forEach((feature: Feature) => {
    if (feature.geometry.type === 'MultiPolygon') {
      const { coordinates } = feature.geometry
      coordinates.forEach((polygon, i) => {
        features.push({
          ...feature,
          id: getId(i + 1, feature.properties?.__mode || 'unset'),
          properties: {
            ...feature.properties,
            __id: undefined,
            __name:
              coordinates.length === 1
                ? feature.properties?.__name || ''
                : `${feature.properties?.__name}__${i}`,
          },
          geometry: {
            ...feature.geometry,
            type: 'Polygon',
            coordinates: polygon,
          },
        })
      })
    } else {
      features.push(feature)
    }
  })
  return {
    ...featureCollection,
    features,
  }
}

export function removeThisPolygon(feature: Feature<MultiPolygon>) {
  const point = {
    type: 'Point',
    coordinates: useStatic.getState().clickedLocation,
  } as const
  const filtered = feature.geometry.coordinates.filter(
    (polygon) =>
      !booleanPointInPolygon(point, {
        type: 'Polygon',
        coordinates: polygon,
      }),
  )
  if (feature.id) {
    useShapes.getState().setters.update('MultiPolygon', feature.id, {
      ...feature,
      geometry: {
        ...feature.geometry,
        coordinates: filtered,
      },
    })
  }
}

export function removeAllOthers(feature: Feature<MultiPolygon>) {
  const point = {
    type: 'Point',
    coordinates: useStatic.getState().clickedLocation,
  } as const
  const found = feature.geometry.coordinates.find((polygon) =>
    booleanPointInPolygon(point, {
      type: 'Polygon',
      coordinates: polygon,
    }),
  )
  if (found) {
    const { add, remove } = useShapes.getState().setters
    remove('MultiPolygon', feature.id)
    add({
      ...feature,
      geometry: {
        ...feature.geometry,
        type: 'Polygon',
        coordinates: found,
      },
    })
  }
}

export function getKey() {
  return Math.random().toString(36).substring(2, 10)
}

export function mpToPoints(geometry: MultiPoint): Feature<Point>[] {
  return (geometry.coordinates || []).map((c, i) => {
    return {
      id: i,
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: c,
      },
      properties: {
        next: geometry.coordinates[
          i === geometry.coordinates.length - 1 ? 0 : i + 1
        ],
      },
    }
  })
}

export function buildShortcutKey(event: React.KeyboardEvent<HTMLDivElement>) {
  let shortcut = ''
  if (!event.key) return shortcut
  if (event.ctrlKey) shortcut += 'ctrl+'
  if (event.altKey) shortcut += 'alt+'
  if (event.shiftKey) shortcut += 'shift+'
  shortcut += event.key.toLowerCase()
  return shortcut
}

export function reverseObject(obj: Record<string, string>) {
  return Object.fromEntries(Object.entries(obj).map(([k, v]) => [v, k]))
}

export function getRouteType(category: Category): KojiRouteModes {
  const { scannerType } = useStatic.getState()
  switch (category) {
    case 'gym':
      return scannerType === 'rdm' ? 'circle_smart_raid' : 'circle_raid'
    case 'pokestop':
      return 'circle_quest'
    default:
      return scannerType === 'rdm' ? 'circle_smart_pokemon' : 'circle_pokemon'
  }
}

export function getCategory(mode: KojiModes): Category {
  switch (mode) {
    case 'circle_quest':
      return 'pokestop'
    case 'circle_raid':
    case 'circle_smart_raid':
      return 'gym'
    default:
      return 'spawnpoint'
  }
}

export function getPolygonColor(id: string) {
  return id.includes('__SCANNER') ? VECTOR_COLORS.GREEN : VECTOR_COLORS.BLUE
}

export function getPointColor(
  id: string,
  type: 'Point' | 'MultiPoint',
  index: number,
) {
  return type === 'Point'
    ? VECTOR_COLORS.RED
    : index === 0
    ? VECTOR_COLORS.PURPLE
    : id.includes('__SCANNER')
    ? VECTOR_COLORS.GREEN
    : VECTOR_COLORS.BLUE
}
