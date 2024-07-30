/* eslint-disable prefer-destructuring */
/* eslint-disable @typescript-eslint/ban-types */
import { create } from 'zustand'
import distance from '@turf/distance'
import type { Position } from 'geojson'

import {
  KojiResponse,
  type Conversions,
  type Feature,
  type FeatureCollection,
} from '@assets/types'
import { convert, fetchWrapper } from '@services/fetches'
import { getCategory } from '@services/utils'

import { UsePersist, usePersist } from './usePersist'
import { UseDbCache, useDbCache } from './useDbCache'
import { useShapes } from './useShapes'

const DEFAULT = { id: 0, source: 'CLIENT', mode: 'unset' } as ReturnType<
  UseDbCache['parseKojiKey']
>

const getProperties = (feature: Feature) => {
  const { geofence } = useDbCache.getState()
  const parsedId =
    typeof feature?.id === 'string'
      ? useDbCache.getState().parseKojiKey(feature.id)
      : { ...DEFAULT }
  const name =
    (feature.properties?.__geofence_id
      ? Object.values(geofence).find(
          (fence) => fence.id === feature.properties.__geofence_id,
        )?.name
      : feature?.properties?.__name) ||
    feature?.properties?.__name ||
    `${parsedId.id}`
  const mode = feature?.properties?.__mode || parsedId.mode || 'unset'
  return { name, mode, source: parsedId.source, id: parsedId.id }
}

export interface UseImportExport {
  code: string
  error: string
  open: 'importPolygon' | 'importRoute' | 'exportPolygon' | 'exportRoute' | ''
  feature: Feature | FeatureCollection
  skipSend: boolean
  fileName: string
  stats: {
    max: number
    total: number
    count: number
    covered: string
    score: number
  }
  importConvert: (geometry?: UsePersist['geometryType']) => Promise<void>
  exportConvert: () => Promise<void>
  updateStats: (writeCode: boolean) => void
  setCode: (code: string | UseImportExport['feature']) => void
  reset: () => void
  fireConvert: (
    mode: 'Import' | 'Export',
    geometry?: UsePersist['geometryType'],
  ) => Promise<void>
}

const DEFAULTS: Omit<
  UseImportExport,
  | 'updateStats'
  | 'setCode'
  | 'reset'
  | 'exportConvert'
  | 'importConvert'
  | 'fireConvert'
> = {
  code: '',
  open: '',
  error: '',
  feature: {
    type: 'FeatureCollection',
    features: [],
  },
  fileName: '',
  skipSend: false,
  stats: {
    max: 0,
    total: 0,
    count: 0,
    covered: '0 / 0',
    score: 0,
  },
}

export const useImportExport = create<UseImportExport>((set, get) => ({
  ...DEFAULTS,
  exportConvert: async () => {
    const { feature } = get()
    const { polygonExportMode, simplifyPolygons } = usePersist.getState()
    const newCode = await convert(feature, polygonExportMode, simplifyPolygons)
    set({
      code:
        typeof newCode === 'string'
          ? newCode
          : JSON.stringify(
              feature.type === 'Feature' &&
                polygonExportMode === 'poracle' &&
                Array.isArray(newCode)
                ? newCode[0]
                : newCode,
              null,
              2,
            ),
      skipSend: true,
    })
  },
  importConvert: async (geometry) => {
    try {
      const cleanCode = get().code.trim()
      const parsed: Conversions =
        cleanCode.startsWith('{') || cleanCode.startsWith('[')
          ? JSON.parse(
              cleanCode.endsWith(',')
                ? cleanCode.substring(0, cleanCode.length - 1)
                : cleanCode,
            )
          : cleanCode
      const geojson = await convert<FeatureCollection>(
        parsed,
        'featureCollection',
        usePersist.getState().simplifyPolygons,
        geometry,
      )
      if (geojson.type === 'FeatureCollection') {
        set({
          feature: {
            ...geojson,
            features: geojson.features.map((f) => ({
              ...f,
              // id: f.id ?? getKey(),
            })),
          },
        })
      }
      set({ error: '' })
    } catch (e) {
      if (e instanceof Error) {
        set({ error: e.message })
      }
    }
  },
  fireConvert: async (mode, geometry) => {
    const { skipSend } = get()
    if (skipSend) return
    if (mode === 'Export') {
      await get().exportConvert()
    } else {
      await get().importConvert(geometry)
    }
    set({ skipSend: false })
  },
  updateStats: async (writeCode) => {
    const { feature, code } = get()
    let max = 0
    let total = 0
    let count = 0
    let covered = '0 / 0'
    let score = 0
    let minLat = Infinity
    let minLon = Infinity
    let maxLat = -Infinity
    let maxLon = -Infinity

    const points: [number, number][] = []

    const update = (pos: Position, j: number, coordinates: Position[]) => {
      const next = j ? coordinates[j + 1] : coordinates.at(-1)
      if (next) {
        const dis = distance(pos, next, { units: 'meters' })
        if (dis > max) max = dis
        total += dis
      }
      points.push([pos[1], pos[0]])
      if (pos[0] < minLon) minLon = pos[0]
      if (pos[0] > maxLon) maxLon = pos[0]
      if (pos[1] < minLat) minLat = pos[1]
      if (pos[1] > maxLat) maxLat = pos[1]
      count++
    }
    if (feature.type === 'Feature') {
      if (feature.geometry.type === 'MultiPoint') {
        feature.geometry.coordinates.forEach(update)
      }
    } else {
      feature.features.forEach((f) => {
        if (f.geometry.type === 'MultiPoint') {
          f.geometry.coordinates.forEach(update)
        }
      })
    }
    const { Polygon, MultiPolygon } = useShapes.getState()
    const { radius, tth, last_seen: raw, min_points } = usePersist.getState()
    const { geofence } = useDbCache.getState()
    const combined = { ...Polygon, ...MultiPolygon }
    const { id, name, mode } =
      feature.type === 'Feature'
        ? getProperties(feature)
        : getProperties(feature.features[0])
    const category = getCategory(mode)
    const sourceArea =
      combined[id] ??
      Object.values(combined).find(
        (feat) =>
          feat.properties?.__name ===
          (feature.type === 'Feature'
            ? feature.properties.__name
            : feature.features[0].properties.__name),
      ) ??
      (Object.values(geofence).find((fence) => fence.name === name)
        ? undefined
        : [
            [
              [minLat, minLon],
              [maxLat, minLon],
              [maxLat, maxLon],
              [minLat, maxLon],
              [minLat, minLon],
            ],
          ])
    const last_seen = typeof raw === 'string' ? new Date(raw) : raw

    set({
      stats: {
        max,
        total,
        count,
        covered,
        score,
      },
      code: writeCode ? JSON.stringify(feature, null, 2) : code,
    })

    if (sourceArea || name) {
      const res = await fetchWrapper<KojiResponse>(
        `/api/v1/calc/route-stats/${category}`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            instance: name,
            mode,
            area: sourceArea,
            clusters: points,
            radius,
            min_points,
            tth,
            last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
          }),
        },
      )

      if (res) {
        const { stats } = res
        max = stats.longest_distance
        total = stats.total_distance
        count = stats.total_clusters
        covered = `${stats.points_covered} / ${stats.total_points}`
        score = stats.mygod_score
      }
    }
    set({
      stats: {
        max,
        total,
        count,
        covered,
        score,
      },
    })
  },
  setCode: (code) => {
    if (typeof code === 'string') {
      set({ code })
    } else {
      set({ code: JSON.stringify(code, null, 2) })
    }
  },
  reset: () => set({ ...DEFAULTS }),
}))
