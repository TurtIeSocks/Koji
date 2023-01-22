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
import { getKey } from '@services/utils'
import union from '@turf/union'
import { Option } from '@assets/types'

export interface UseShapes {
  test: boolean
  activeRoute: string
  newRouteCount: number
  kojiRefCache: Record<string, Option>
  remoteCache: Record<string, Feature>
  firstPoint: keyof UseShapes['Point'] | null
  lastPoint: keyof UseShapes['Point'] | null
  Point: Record<number | string, Feature<Point>>
  MultiPoint: Record<number | string, Feature<MultiPoint>>
  LineString: Record<number | string, Feature<LineString>>
  MultiLineString: Record<number | string, Feature<MultiLineString>>
  Polygon: Record<number | string, Feature<Polygon>>
  MultiPolygon: Record<number | string, Feature<MultiPolygon>>
  GeometryCollection: Record<number | string, Feature<GeometryCollection>>
  combined: Record<string, boolean>
  getters: {
    getFirst: () => Feature<Point> | null
    getLast: () => Feature<Point> | null
    getGeojson: (types?: GeoJsonTypes[]) => FeatureCollection
    getNewPointId: (id: number | string) => number | string
  }
  setters: {
    combine: () => void
    setFromCollection: (
      collection: FeatureCollection,
      source?: '__SCANNER' | '',
    ) => void
    add: (
      feature: Feature | Feature[],
      source?: '__SCANNER' | '' | '__KOJI',
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
    updateProperty: <
      T extends Exclude<GeoJsonTypes, 'Feature' | 'FeatureCollection'>,
      U extends number | string,
    >(
      key: T,
      id: U,
      propertyKey: string,
      value: unknown,
    ) => void
    activeRoute: (newId?: string) => void
    splitLine: <U extends keyof UseShapes['LineString']>(id: U) => void
  }
  setShapes: <
    T extends keyof Omit<UseShapes, 'getters' | 'setters' | 'setShapes'>,
  >(
    key: T,
    init: UseShapes[T] | ((prev: UseShapes[T]) => UseShapes[T]),
  ) => void
}

export const useShapes = create<UseShapes>((set, get) => ({
  activeRoute: 'new_route_0',
  remoteCache: {},
  kojiRefCache: {},
  newRouteCount: 0,
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
  combined: {},
  getters: {
    getNewPointId: (id) => {
      let newId = id
      while (get().Point[newId]) {
        newId = +newId + 1
      }
      return newId
    },
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
      if (!feature) return
      if (Array.isArray(feature)) {
        feature.forEach((f) => get().setters.add(f, source))
      } else {
        if (source) {
          set((state) => ({
            remoteCache: {
              ...state.remoteCache,
              [`${feature.properties?.__name}__${
                feature.properties?.__type || ''
              }`]: feature,
            },
          }))
        }
        const id =
          feature.id ??
          `${feature.properties?.__name}__${
            feature.properties?.__type || ''
          }${source}`
        set((state) => ({
          [feature.geometry.type]: {
            ...state[feature.geometry.type],
            [id]: { ...feature, id },
          },
        }))
      }
    },
    activeRoute: (newId) => {
      if (!newId) return set({ activeRoute: 'new_route_0' })
      const { activeRoute, Point, MultiPoint } = get()
      if (activeRoute !== newId) {
        const newMultiPoint: Record<string | number, Feature<MultiPoint>> = {
          ...MultiPoint,
        }
        if (Object.values(Point).length) {
          const { forward, backward, ...rest } =
            Object.values(Point)?.[0]?.properties || {}

          newMultiPoint[activeRoute] = {
            type: 'Feature',
            id: activeRoute,
            properties: MultiPoint[activeRoute] || rest,
            geometry: {
              type: 'MultiPoint',
              coordinates: Object.values(Point).map(
                (p) => p.geometry.coordinates,
              ),
            },
          }
        }
        const newPoint: Record<
          string | number,
          Feature<Point>
        > = Object.fromEntries(
          Object.values(MultiPoint[newId]?.geometry?.coordinates || {}).map(
            (point, i) => [
              i * 10,
              {
                ...MultiPoint[newId],
                properties: {
                  ...MultiPoint[newId]?.properties,
                  forward: i
                    ? i ===
                      (MultiPoint[newId]?.geometry?.coordinates?.length || 0) -
                        1
                      ? 0
                      : i * 10 + 10
                    : 10,
                  backward: i
                    ? i * 10 - 10
                    : (MultiPoint[newId]?.geometry?.coordinates?.length || 0) *
                        10 -
                      10,
                },
                id: i * 10,
                geometry: { type: 'Point', coordinates: point },
              },
            ],
          ),
        )
        const newLineString: Record<string | number, Feature<LineString>> = {}
        Object.entries(newPoint).forEach(([key, point], i) => {
          const prevPoint = i
            ? newPoint[+key - 10]
            : newPoint[Object.values(newPoint).at(-1)?.id || 0]
          const id = `${prevPoint.id}_${point.id}`
          newLineString[id] = {
            type: 'Feature',
            id,
            properties: {
              start: prevPoint.id,
              end: point.id,
            },
            geometry: {
              type: 'LineString',
              coordinates: [
                prevPoint.geometry.coordinates,
                point.geometry.coordinates,
              ],
            },
          }
        })
        delete newMultiPoint[newId]
        set({
          firstPoint: Object.keys(newPoint).at(0) || null,
          lastPoint: Object.keys(newPoint).at(-1) || null,
          activeRoute: newId,
          Point: newPoint,
          LineString: newLineString,
          MultiPoint: newMultiPoint,
        })
      }
    },
    combine: () => {
      const {
        combined,
        Polygon,
        MultiPolygon,
        setters: { remove, add },
      } = get()
      let newPoly: Feature<Polygon | MultiPolygon> = {
        geometry: { type: 'MultiPolygon', coordinates: [] },
        type: 'Feature',
        properties: {},
        id: getKey(),
      }
      Object.entries(combined).forEach(([key, value]) => {
        if (value) {
          const isPolygon = Polygon[key]
          const polygon = isPolygon ? Polygon[key] : MultiPolygon[key]
          if (polygon) {
            remove(isPolygon ? 'Polygon' : 'MultiPolygon', key)

            const possiblyNew = union(newPoly, polygon)
            if (possiblyNew) {
              newPoly = {
                ...newPoly,
                properties: {
                  ...newPoly.properties,
                  ...possiblyNew.properties,
                  __koji_id: undefined,
                  __name: undefined,
                  __type: undefined,
                },
                geometry: possiblyNew.geometry,
              }
            }
          }
        }
      })
      if (newPoly.geometry.type === 'Polygon') {
        newPoly.geometry.coordinates = [newPoly.geometry.coordinates[0]]
      }
      add(newPoly)

      set({ combined: {} })
    },
    splitLine: (id) => {
      if (typeof id === 'string') {
        const { Point, setters, getters } = get()

        const [pointKeyOne, pointKeyTwo] = id.split('_')
        const firstPoint = Point[pointKeyOne]
        const secondPoint = Point[pointKeyTwo]
        if (firstPoint?.id !== undefined && secondPoint?.id !== undefined) {
          const center: Feature<Point> = {
            id: getters.getNewPointId(
              Math.ceil((+firstPoint.id + +secondPoint.id) / 2),
            ),
            type: 'Feature',
            properties: {
              ...firstPoint.properties,
              forward: secondPoint.id,
              backward: firstPoint.id,
            },
            geometry: {
              type: 'Point',
              coordinates: [
                (firstPoint.geometry.coordinates[0] +
                  secondPoint.geometry.coordinates[0]) /
                  2,
                (firstPoint.geometry.coordinates[1] +
                  secondPoint.geometry.coordinates[1]) /
                  2,
              ],
            },
          }
          setters.add(center)
          setters.update('Point', firstPoint.id, {
            ...firstPoint,
            properties: { ...firstPoint.properties, forward: center.id },
          })
          setters.update('Point', secondPoint.id, {
            ...secondPoint,
            properties: { ...secondPoint.properties, backward: center.id },
          })
          const firstLine: Feature<LineString> = {
            id: `${firstPoint.id}_${center.id}`,
            type: 'Feature',
            properties: {
              start: firstPoint.id,
              end: center.id,
            },
            geometry: {
              type: 'LineString',
              coordinates: [
                firstPoint.geometry.coordinates.slice(),
                center.geometry.coordinates.slice(),
              ],
            },
          }
          const secondLine: Feature<LineString> = {
            id: `${center.id}_${secondPoint.id}`,
            type: 'Feature',
            properties: {
              start: center.id,
              end: secondPoint.id,
            },
            geometry: {
              type: 'LineString',
              coordinates: [
                center.geometry.coordinates.slice(),
                secondPoint.geometry.coordinates.slice(),
              ],
            },
          }
          setters.add(firstLine)
          setters.add(secondLine)
          setters.remove('LineString', id)
        }
      }
    },
    update: (key, id, feature) => {
      set((state) => {
        const newState = {
          [key]: { ...state[key] },
          [feature.geometry.type]: { ...state[feature.geometry.type] },
        }
        if (key === 'Point' && feature.geometry.type === 'Point') {
          newState[key][id] = feature
          newState.LineString = { ...state.LineString }
          const firstPoint = state.Point[state.Point[id].properties?.forward]
          const firstLine = Object.values(state.LineString).find(
            (line) => line.properties?.end === firstPoint?.id,
          )
          const secondPoint = state.Point[state.Point[id].properties?.backward]
          const secondLine = Object.values(state.LineString).find(
            (line) => line.properties?.start === secondPoint?.id,
          )

          if (firstLine?.id !== undefined) {
            firstLine.geometry.coordinates = [
              feature.geometry.coordinates,
              firstPoint?.geometry.coordinates,
            ]
            newState.LineString[firstLine?.id] = firstLine
          }
          if (secondLine?.id !== undefined) {
            secondLine.geometry.coordinates = [
              secondPoint?.geometry.coordinates,
              feature.geometry.coordinates,
            ]
            newState.LineString[secondLine?.id] = secondLine
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
        return { ...newState, test: !state.test }
      })
    },
    updateProperty: (key, id, property, value) => {
      set((prev) => ({
        [key]: {
          ...prev[key],
          [id]: {
            ...prev[key][id],
            properties: {
              ...prev[key][id].properties,
              [property]: value,
            },
          },
        },
      }))
    },
    remove: (key, id) => {
      // todo fix types
      set((state) => {
        const newState = {
          lastPoint: state.lastPoint,
          firstPoint: state.firstPoint,
          [key]: { ...state[key] },
        }
        if (id !== undefined) {
          if (key === 'Point') {
            newState.LineString = { ...state.LineString }
            const val = state[key][id] // Point to delete

            const firstPoint = state.Point[val.properties?.forward]
            const firstLine = Object.values(state.LineString).find(
              (line) => line.properties?.end === firstPoint?.id,
            )
            const secondPoint = state.Point[val.properties?.backward]
            const secondLine = Object.values(state.LineString).find(
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

            if (
              firstLine?.id !== undefined &&
              secondLine?.id !== undefined &&
              val.properties?.forward !== undefined &&
              val.properties?.backward !== undefined
            ) {
              if (Object.keys(newState.Point || {}).length > 2) {
                newState.LineString[
                  `${secondLine.properties?.start}_${firstLine.properties?.end}`
                ] = {
                  type: 'Feature',
                  id: `${secondLine.properties?.start}_${firstLine.properties?.end}`,
                  geometry: {
                    type: 'LineString',
                    coordinates: [
                      (
                        (newState[key] as UseShapes['Point'])[
                          val.properties?.backward
                        ] as Feature<Point>
                      ).geometry.coordinates,
                      (
                        (newState[key] as UseShapes['Point'])[
                          val.properties?.forward
                        ] as Feature<Point>
                      ).geometry.coordinates,
                    ],
                  },
                  properties: {
                    start: secondLine.properties?.start,
                    end: firstLine.properties?.end,
                  },
                }
              } else {
                newState.LineString = {}
              }
              ;(newState[key] as UseShapes['Point'])[val.properties?.forward] =
                {
                  ...firstPoint,
                  properties: {
                    ...firstPoint.properties,
                    backward: secondLine.properties?.start,
                  },
                }
              ;(newState[key] as UseShapes['Point'])[val.properties?.backward] =
                {
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
            const newGeometry = (newState[key] as UseShapes['MultiPoint'])[
              parent
            ].geometry
            if (newGeometry.type === 'MultiPoint') {
              newGeometry.coordinates.splice(+child, 1)
              ;(newState[key] as UseShapes['MultiPoint'])[parent].geometry =
                newGeometry
            }
          }
          if (newState[key]) {
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            delete newState[key][id]
          }
        } else {
          newState[key] = {}
          if (key === 'Point') {
            newState.LineString = {}
            newState.firstPoint = null
            newState.lastPoint = null
          }
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
          `${feature.properties?.__name}${feature.properties?.__type}${source}`
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
