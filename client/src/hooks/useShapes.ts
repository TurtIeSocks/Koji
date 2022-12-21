import create from 'zustand'
import type {
  Feature,
  Point,
  MultiPoint,
  LineString,
  MultiLineString,
  Polygon,
  MultiPolygon,
  FeatureCollection,
  GeoJsonTypes,
} from 'geojson'
import { filterImports } from '@services/utils'

export interface UseShapes {
  test: boolean
  firstPoint: keyof UseShapes['points'] | null
  lastPoint: keyof UseShapes['points'] | null
  points: Record<number | string, Feature<Point>>
  multiPoints: Record<number | string, Feature<MultiPoint>>
  lineStrings: Record<number | string, Feature<LineString>>
  multiLineStrings: Record<number | string, Feature<MultiLineString>>
  polygons: Record<number | string, Feature<Polygon>>
  multiPolygons: Record<number | string, Feature<MultiPolygon>>
  getters: {
    getFirst: () => Feature<Point> | null
    getLast: () => Feature<Point> | null
    getGeojson: (types?: GeoJsonTypes[]) => FeatureCollection
  }
  setters: {
    setFromCollection: (
      collection: FeatureCollection,
      source?: 'instances' | 'geofences' | '',
    ) => void
    remove: (
      key:
        | 'points'
        | 'multiPoints'
        | 'lineStrings'
        | 'multiLineStrings'
        | 'polygons'
        | 'multiPolygons',
      id?: number | string,
    ) => void
    update: (
      key:
        | 'points'
        | 'multiPoints'
        | 'lineStrings'
        | 'multiLineStrings'
        | 'polygons'
        | 'multiPolygons',
      id: number | string,
      feature: Feature,
    ) => void
  }
  setShapes: <
    T extends keyof Omit<UseShapes, 'getters' | 'setters' | 'setShapes'>,
  >(
    key: T,
    init: UseShapes[T] | ((prev: UseShapes[T]) => UseShapes[T]),
  ) => void
}

export const useShapes = create<UseShapes>((set, get) => ({
  test: false,
  firstPoint: null,
  lastPoint: null,
  points: {},
  multiPoints: {},
  lineStrings: {},
  multiLineStrings: {},
  polygons: {},
  multiPolygons: {},
  getters: {
    getFirst: () => {
      const { firstPoint, points } = get()
      return firstPoint ? points[firstPoint] : null
    },
    getLast: () => {
      const { lastPoint, points } = get()
      return lastPoint ? points[lastPoint] : null
    },
    getGeojson: (types) => {
      const {
        points,
        multiPoints,
        lineStrings,
        multiLineStrings,
        polygons,
        multiPolygons,
      } = get()
      return {
        type: 'FeatureCollection',
        features: [
          ...(!types || types.includes('Point') ? Object.values(points) : []),
          ...(!types || types.includes('MultiPoint')
            ? Object.values(multiPoints)
            : []),
          ...(!types || types.includes('LineString')
            ? Object.values(lineStrings)
            : []),
          ...(!types || types.includes('MultiLineString')
            ? Object.values(multiLineStrings)
            : []),
          ...(!types || types.includes('Polygon')
            ? Object.values(polygons)
            : []),
          ...(!types || types.includes('MultiPolygon')
            ? Object.values(multiPolygons)
            : []),
        ],
      }
    },
  },
  setShapes: (key, newValue) => {
    set((state) => ({
      [key]: typeof newValue === 'function' ? newValue(state[key]) : newValue,
    }))
  },
  setters: {
    update: (key, id, feature) => {
      set((state) => {
        const newState = { ...state }
        if (key === 'points' && feature.geometry.type === 'Point') {
          newState.points[id] = feature as Feature<Point>

          const firstPoint =
            newState.points[newState.points[id].properties?.forward]
          const firstLine = Object.values(newState.lineStrings).find(
            (line) => line.properties?.end === firstPoint?.id,
          )
          const secondPoint =
            newState.points[newState.points[id].properties?.backward]
          const secondLine = Object.values(newState.lineStrings).find(
            (line) => line.properties?.start === secondPoint?.id,
          )

          if (firstLine?.id) {
            firstLine.geometry.coordinates = [
              feature.geometry.coordinates,
              firstPoint?.geometry.coordinates,
            ]
            newState.lineStrings[firstLine?.id] = firstLine
          }
          if (secondLine?.id) {
            secondLine.geometry.coordinates = [
              secondPoint?.geometry.coordinates,
              feature.geometry.coordinates,
            ]
            newState.lineStrings[secondLine?.id] = secondLine
          }
        } else if (
          key === 'multiPoints' &&
          feature.geometry.type === 'Point' &&
          typeof id === 'string'
        ) {
          const [parent, child] = id.split('___')
          newState.multiPoints[parent].geometry.coordinates.splice(
            +child,
            1,
            feature.geometry.coordinates,
          )
        }
        return { ...newState, test: !state.test }
      })
    },
    remove: (key, id) => {
      set((state) => {
        const newState = { ...state }
        if (id) {
          if (key === 'points') {
            const val = newState[key][id] // Point to delete

            const firstPoint = newState.points[val.properties?.forward]
            const firstLine = Object.values(newState.lineStrings).find(
              (line) => line.properties?.end === firstPoint?.id,
            )
            const secondPoint = newState.points[val.properties?.backward]
            const secondLine = Object.values(newState.lineStrings).find(
              (line) => line.properties?.start === secondPoint?.id,
            )

            if (val?.id === state.firstPoint && firstPoint?.id) {
              // If the point to delete was the first point
              newState.firstPoint = firstPoint.id
            }
            if (val?.id === state.lastPoint && secondPoint?.id) {
              // If the point to delete is the last point
              newState.lastPoint = secondPoint.id
            }

            if (firstLine?.id && secondLine?.id) {
              newState.lineStrings[
                `${secondLine.properties?.start}_${firstLine.properties?.end}`
              ] = {
                type: 'Feature',
                id: `${secondLine.properties?.start}_${firstLine.properties?.end}`,
                geometry: {
                  type: 'LineString',
                  coordinates: [
                    newState.points[val.properties?.backward].geometry
                      .coordinates,
                    newState.points[val.properties?.forward].geometry
                      .coordinates,
                  ],
                },
                properties: {
                  start: secondLine.properties?.start,
                  end: firstLine.properties?.end,
                },
              }
              newState.points[val.properties?.forward] = {
                ...firstPoint,
                properties: {
                  ...firstPoint.properties,
                  backward: secondLine.properties?.start,
                },
              }
              newState.points[val.properties?.backward] = {
                ...secondPoint,
                properties: {
                  ...secondPoint.properties,
                  forward: firstLine.properties?.end,
                },
              }
              delete newState.lineStrings[firstLine?.id]
              delete newState.lineStrings[secondLine?.id]
            }
          } else if (key === 'multiPoints' && typeof id === 'string') {
            const [parent, child] = id.split('___')
            newState.multiPoints[parent].geometry.coordinates.splice(+child, 1)
          }
          if (newState[key][id]) {
            delete newState[key][id]
          }
        } else {
          newState[key] = {}
        }
        return {
          ...newState,
          test: !state.test,
        }
      })
    },
    setFromCollection: (collection, source = '') => {
      const points: UseShapes['points'] = {}
      const multiPoints: UseShapes['multiPoints'] = {}
      const lineStrings: UseShapes['lineStrings'] = {}
      const multiLineStrings: UseShapes['multiLineStrings'] = {}
      const polygons: UseShapes['polygons'] = {}
      const multiPolygons: UseShapes['multiPolygons'] = {}

      collection.features.forEach((feature) => {
        const id =
          feature.id ??
          `${feature.properties?.name}${feature.properties?.type}${source}`
        switch (feature.geometry.type) {
          case 'Point':
            points[id] = { ...feature, id } as Feature<Point>
            break
          case 'MultiPoint':
            multiPoints[id] = { ...feature, id } as Feature<MultiPoint>
            break
          case 'LineString':
            lineStrings[id] = { ...feature, id } as Feature<LineString>
            break
          case 'MultiLineString':
            multiLineStrings[id] = {
              ...feature,
              id,
            } as Feature<MultiLineString>
            break
          case 'Polygon':
            polygons[id] = { ...feature, id } as Feature<Polygon>
            break
          case 'MultiPolygon':
            multiPolygons[id] = { ...feature, id } as Feature<MultiPolygon>
            break
          default:
            break
        }
      })
      set((state) => ({
        points: {
          ...filterImports(state.points),
          ...points,
        },
        multiPoints: { ...filterImports(state.multiPoints), ...multiPoints },
        lineStrings: { ...filterImports(state.lineStrings), ...lineStrings },
        multiLineStrings: {
          ...filterImports(state.multiLineStrings),
          ...multiLineStrings,
        },
        polygons: { ...filterImports(state.polygons), ...polygons },
        multiPolygons: {
          ...filterImports(state.multiPolygons),
          ...multiPolygons,
        },
      }))
    },
  },
}))
