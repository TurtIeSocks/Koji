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
  GeometryCollection,
} from 'geojson'

export interface UseShapes {
  test: boolean
  firstPoint: keyof UseShapes['Point'] | null
  lastPoint: keyof UseShapes['Point'] | null
  Point: Record<number | string, Feature<Point>>
  MultiPoint: Record<number | string, Feature<MultiPoint>>
  LineString: Record<number | string, Feature<LineString>>
  MultiLineString: Record<number | string, Feature<MultiLineString>>
  Polygon: Record<number | string, Feature<Polygon>>
  MultiPolygon: Record<number | string, Feature<MultiPolygon>>
  GeometryCollection: Record<number | string, Feature<GeometryCollection>>
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
    add: (
      feature: Feature | Feature[],
      source?: 'instances' | 'geofences' | '',
    ) => void
    remove: (
      key: Exclude<GeoJsonTypes, 'Feature' | 'FeatureCollection'>,
      id?: number | string,
    ) => void
    update: <
      T extends Exclude<GeoJsonTypes, 'Feature' | 'FeatureCollection'>,
      U extends number | string,
    >(
      key: T,
      id: U,
      feature: UseShapes[T][U],
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
  Point: {},
  MultiPoint: {},
  LineString: {},
  MultiLineString: {},
  Polygon: {},
  MultiPolygon: {},
  GeometryCollection: {},
  getters: {
    getFirst: () => {
      const { firstPoint, Point: points } = get()
      return firstPoint ? points[firstPoint] : null
    },
    getLast: () => {
      const { lastPoint, Point: points } = get()
      return lastPoint ? points[lastPoint] : null
    },
    getGeojson: (types) => {
      const {
        Point: points,
        MultiPoint: multiPoints,
        LineString: lineStrings,
        MultiLineString: multiLineStrings,
        Polygon: polygons,
        MultiPolygon: multiPolygons,
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
    add: (feature, source = '') => {
      if (Array.isArray(feature)) {
        feature.forEach((f) => get().setters.add(f, source))
      } else {
        const id =
          feature.id ??
          `${feature.properties?.name}${feature.properties?.type}${source}`
        set((state) => ({
          [feature.geometry.type]: {
            ...state[feature.geometry.type],
            [id]: { ...feature, id },
          },
        }))
      }
    },
    update: (key, id, feature) => {
      set((state) => {
        const newState = {
          [key]: { ...state[key] },
          [feature.geometry.type]: { ...state[feature.geometry.type] },
        }
        if (newState[key]) {
          if (key === 'Point' && feature.geometry.type === 'Point') {
            newState[key][id] = feature

            const firstPoint = state.Point[state.Point[id].properties?.forward]
            const firstLine = Object.values(state.LineString).find(
              (line) => line.properties?.end === firstPoint?.id,
            )
            const secondPoint =
              state.Point[state.Point[id].properties?.backward]
            const secondLine = Object.values(state.LineString).find(
              (line) => line.properties?.start === secondPoint?.id,
            )

            if (firstLine?.id) {
              firstLine.geometry.coordinates = [
                feature.geometry.coordinates,
                firstPoint?.geometry.coordinates,
              ]
              newState[key][firstLine?.id] = firstLine
            }
            if (secondLine?.id) {
              secondLine.geometry.coordinates = [
                secondPoint?.geometry.coordinates,
                feature.geometry.coordinates,
              ]
              newState[key][secondLine?.id] = secondLine
            }
          } else if (
            key === 'MultiPoint' &&
            feature.geometry.type === 'Point' &&
            typeof id === 'string'
          ) {
            const [parent, child] = id.split('___')
            const newGeometry = newState[key][parent].geometry
            if (newGeometry.type === 'MultiPoint') {
              newGeometry.coordinates.splice(
                +child,
                1,
                feature.geometry.coordinates,
              )
              newState[key][parent].geometry = newGeometry
            }
          } else if (key !== feature.geometry.type) {
            const newId = feature.properties?.leafletId || feature.id
            newState[feature.geometry.type][newId] = {
              ...feature,
              id: newId,
              properties: { ...feature.properties },
            }
            delete newState[key][id]
          } else if (
            feature.properties?.leafletId &&
            id !== feature.properties?.leafletId
          ) {
            const { leafletId, ...rest } = feature.properties || {}
            newState[key][leafletId] = {
              ...feature,
              id: leafletId,
              properties: rest,
            }
            delete newState[key][id]
          } else {
            newState[key][id] = feature
          }
        }
        return { ...newState, test: !state.test }
      })
    },
    remove: (key, id) => {
      set((state) => {
        const newState = { ...state }
        if (id) {
          if (key === 'Point') {
            const val = newState[key][id] // Point to delete

            const firstPoint = newState.Point[val.properties?.forward]
            const firstLine = Object.values(newState.LineString).find(
              (line) => line.properties?.end === firstPoint?.id,
            )
            const secondPoint = newState.Point[val.properties?.backward]
            const secondLine = Object.values(newState.LineString).find(
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
              newState.LineString[
                `${secondLine.properties?.start}_${firstLine.properties?.end}`
              ] = {
                type: 'Feature',
                id: `${secondLine.properties?.start}_${firstLine.properties?.end}`,
                geometry: {
                  type: 'LineString',
                  coordinates: [
                    newState.Point[val.properties?.backward].geometry
                      .coordinates,
                    newState.Point[val.properties?.forward].geometry
                      .coordinates,
                  ],
                },
                properties: {
                  start: secondLine.properties?.start,
                  end: firstLine.properties?.end,
                },
              }
              newState.Point[val.properties?.forward] = {
                ...firstPoint,
                properties: {
                  ...firstPoint.properties,
                  backward: secondLine.properties?.start,
                },
              }
              newState.Point[val.properties?.backward] = {
                ...secondPoint,
                properties: {
                  ...secondPoint.properties,
                  forward: firstLine.properties?.end,
                },
              }
              delete newState.LineString[firstLine?.id]
              delete newState.LineString[secondLine?.id]
            }
          } else if (key === 'MultiPoint' && typeof id === 'string') {
            const [parent, child] = id.split('___')
            newState.MultiPoint[parent].geometry.coordinates.splice(+child, 1)
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
      const Point: UseShapes['Point'] = {}
      const MultiPoint: UseShapes['MultiPoint'] = {}
      const LineString: UseShapes['LineString'] = {}
      const MultiLineString: UseShapes['MultiLineString'] = {}
      const Polygon: UseShapes['Polygon'] = {}
      const MultiPolygon: UseShapes['MultiPolygon'] = {}

      collection.features.forEach((feature) => {
        const id =
          feature.id ??
          `${feature.properties?.name}${feature.properties?.type}${source}`
        switch (feature.geometry.type) {
          case 'Point':
            Point[id] = { ...feature, id } as Feature<Point>
            break
          case 'MultiPoint':
            MultiPoint[id] = { ...feature, id } as Feature<MultiPoint>
            break
          case 'LineString':
            LineString[id] = { ...feature, id } as Feature<LineString>
            break
          case 'MultiLineString':
            MultiLineString[id] = {
              ...feature,
              id,
            } as Feature<MultiLineString>
            break
          case 'Polygon':
            Polygon[id] = { ...feature, id } as Feature<Polygon>
            break
          case 'MultiPolygon':
            MultiPolygon[id] = { ...feature, id } as Feature<MultiPolygon>
            break
          default:
            break
        }
      })
      set({
        Point,
        MultiPoint,
        LineString,
        MultiLineString,
        Polygon,
        MultiPolygon,
      })
    },
  },
}))
