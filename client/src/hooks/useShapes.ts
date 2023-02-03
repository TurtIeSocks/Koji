/* eslint-disable @typescript-eslint/ban-ts-comment */
import create from 'zustand'
import type {
  Point,
  MultiPoint,
  LineString,
  MultiLineString,
  Polygon,
  MultiPolygon,
  GeoJsonTypes,
  GeometryCollection,
} from 'geojson'
import { getKey } from '@services/utils'
import union from '@turf/union'
import type {
  DbOption,
  Feature,
  FeatureCollection,
  GeometryTypes,
  KojiKey,
} from '@assets/types'
import { useDbCache } from './useDbCache'

const { setRecord } = useDbCache.getState()

export interface UseShapes {
  test: boolean
  activeRoute: string
  newRouteCount: number
  kojiRefCache: Record<string, DbOption>
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
    getNewPointId: (id: number) => number
  }
  setters: {
    combine: () => void
    setFromCollection: (
      collection: FeatureCollection,
      source?: '__SCANNER' | '',
    ) => void
    add: (
      feature?: Feature | Feature[],
      source?: '__SCANNER' | '' | '__KOJI',
    ) => void
    remove: (key: GeometryTypes, id?: number | string) => void
    update: <T extends GeometryTypes, U extends number | string>(
      key: T,
      id: U,
      feature: UseShapes[T][U],
    ) => void
    updateProperty: <T extends GeometryTypes, U extends number | string>(
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
          setRecord('feature', feature.id as KojiKey, feature)
        }
        const id =
          feature.id ??
          `${feature.properties?.__name}__${
            feature.properties?.__mode || ''
          }${source}`
        set((state) => ({
          [feature.geometry.type]: {
            ...state[feature.geometry.type],
            [id]: { ...feature, id },
          },
        }))
      }
    },
    activeRoute: (incomingId) => {
      const newId = incomingId || 'new_route_0'

      const { activeRoute, Point, MultiPoint } = get()
      if (activeRoute !== newId) {
        const newMultiPoint: Record<string | number, Feature<MultiPoint>> = {
          ...MultiPoint,
        }
        if (Object.values(Point).length) {
          const { __forward, __backward, ...rest } =
            Object.values(Point)?.[0]?.properties || {}

          newMultiPoint[activeRoute] = {
            type: 'Feature',
            id: activeRoute,
            properties: MultiPoint[activeRoute]?.properties || rest,
            geometry: {
              type: 'MultiPoint',
              coordinates: Object.values(Point).map(
                (p) => p.geometry.coordinates,
              ),
            },
          }
        }
        const newPoints: Record<
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
                  __multipoint_id: MultiPoint[newId]?.id as KojiKey,
                  __forward: i
                    ? i ===
                      (MultiPoint[newId]?.geometry?.coordinates?.length || 0) -
                        1
                      ? 0
                      : i * 10 + 10
                    : 10,
                  __backward: i
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
        Object.entries(newPoints).forEach(([key, point], i) => {
          const prevPoint = i
            ? newPoints[+key - 10]
            : newPoints[Object.values(newPoints).at(-1)?.id || 0]
          const id = `${+prevPoint.id}__${+point.id}` as const
          newLineString[id] = {
            type: 'Feature',
            id,
            properties: {
              __start: +prevPoint.id,
              __end: +point.id,
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
          firstPoint: Object.keys(newPoints).at(0) || null,
          lastPoint: Object.keys(newPoints).at(-1) || null,
          activeRoute: newId,
          Point: newPoints,
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
                  __id: undefined,
                  __name: undefined,
                  __mode: undefined,
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

        const [pointKeyOne, pointKeyTwo] = id.split('__')
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
              __forward: +secondPoint.id,
              __backward: +firstPoint.id,
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
            properties: { ...firstPoint.properties, __forward: +center.id },
          })
          setters.update('Point', secondPoint.id, {
            ...secondPoint,
            properties: { ...secondPoint.properties, __backward: +center.id },
          })
          const firstLine: Feature<LineString> = {
            id: `${+firstPoint.id}__${+center.id}`,
            type: 'Feature',
            properties: {
              __start: +firstPoint.id,
              __end: +center.id,
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
            id: `${+center.id}__${+secondPoint.id}`,
            type: 'Feature',
            properties: {
              __start: +center.id,
              __end: +secondPoint.id,
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
        const { type } = feature.geometry
        const newState = {
          [key]: { ...state[key] },
          [type]: { ...state[type] },
        }
        if (key === 'Point' && type === 'Point') {
          newState[key][id] = feature
          newState.LineString = { ...state.LineString }
          const firstPoint =
            state.Point[state.Point[id].properties?.__forward || '']
          const firstLine = Object.values(state.LineString).find(
            (line) => line.properties?.__end === firstPoint?.id,
          )
          const secondPoint =
            state.Point[state.Point[id].properties?.__backward || '']
          const secondLine = Object.values(state.LineString).find(
            (line) => line.properties?.__start === secondPoint?.id,
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
          type === 'Point' &&
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
        } else if (key !== type) {
          const newId = feature.properties?.__leafletId || feature.id
          // @ts-ignore
          newState[feature.geometry.type][newId] = {
            ...feature,
            id: newId,
            properties: { ...feature.properties },
          }
          delete newState[key][id]
        } else if (
          feature.properties?.__leafletId &&
          id !== feature.properties?.__leafletId
        ) {
          const { __leafletId, ...rest } = feature.properties || {}
          if (__leafletId) {
            // @ts-ignore
            newState[key][__leafletId] = {
              ...feature,
              id: __leafletId,
              properties: rest,
            }
            delete newState[key][id]
          }
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

            const firstPoint = state.Point[val.properties?.__forward || '']
            const firstLine = Object.values(state.LineString).find(
              (line) => line.properties?.__end === firstPoint?.id,
            )
            const secondPoint = state.Point[val.properties?.__backward || '']
            const secondLine = Object.values(state.LineString).find(
              (line) => line.properties?.__start === secondPoint?.id,
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
              val.properties?.__forward !== undefined &&
              val.properties?.__backward !== undefined
            ) {
              if (Object.keys(newState.Point || {}).length > 2) {
                newState.LineString[
                  `${secondLine.properties?.__start}__${firstLine.properties?.__end}`
                ] = {
                  type: 'Feature',
                  id: `${secondLine.properties?.__start || 0}__${
                    firstLine.properties?.__end || 0
                  }`,
                  geometry: {
                    type: 'LineString',
                    coordinates: [
                      (
                        (newState[key] as UseShapes['Point'])[
                          val.properties?.__backward
                        ] as Feature<Point>
                      ).geometry.coordinates,
                      (
                        (newState[key] as UseShapes['Point'])[
                          val.properties?.__forward
                        ] as Feature<Point>
                      ).geometry.coordinates,
                    ],
                  },
                  properties: {
                    __start: secondLine.properties?.__start,
                    __end: firstLine.properties?.__end,
                  },
                }
              } else {
                newState.LineString = {}
              }
              ;(newState[key] as UseShapes['Point'])[
                val.properties?.__forward
              ] = {
                ...firstPoint,
                properties: {
                  ...firstPoint.properties,
                  __backward: secondLine.properties?.__start,
                },
              }
              ;(newState[key] as UseShapes['Point'])[
                val.properties?.__backward
              ] = {
                ...secondPoint,
                properties: {
                  ...secondPoint.properties,
                  __forward: firstLine.properties?.__end,
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
          `${feature.properties?.__name}${feature.properties?.__mode}${source}`
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
