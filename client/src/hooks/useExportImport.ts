/* eslint-disable @typescript-eslint/ban-types */
import create from 'zustand'
import distance from '@turf/distance'

import type { Conversions, Feature, FeatureCollection } from '@assets/types'
import { convert } from '@services/fetches'
import { UsePersist, usePersist } from './usePersist'

export interface UseImportExport {
  code: string
  error: string
  open: 'importPolygon' | 'importRoute' | 'exportPolygon' | 'exportRoute' | ''
  feature: Feature | FeatureCollection
  stats: {
    max: number
    total: number
    count: number
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
  stats: {
    max: 0,
    total: 0,
    count: 0,
  },
}

export const useImportExport = create<UseImportExport>((set, get) => ({
  ...DEFAULTS,
  exportConvert: async () => {
    const { feature, setCode } = get()
    const { polygonExportMode, simplifyPolygons } = usePersist.getState()
    convert(feature, polygonExportMode, simplifyPolygons).then((newCode) => {
      setCode(
        typeof newCode === 'string'
          ? newCode
          : JSON.stringify(
              polygonExportMode === 'poracle' && Array.isArray(newCode)
                ? newCode[0]
                : newCode,
              null,
              2,
            ),
      )
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
      await convert<FeatureCollection>(
        parsed,
        'featureCollection',
        usePersist.getState().simplifyPolygons,
        geometry,
      ).then((geojson) => {
        if (geojson.type === 'FeatureCollection') {
          useImportExport.setState({
            feature: {
              ...geojson,
              features: geojson.features.map((f) => ({
                ...f,
                id:
                  f.id ??
                  `${f.properties?.__name || f.geometry.type}__${
                    f.properties?.__mode || 'Unset'
                  }`,
              })),
            },
          })
        }
      })
      useImportExport.setState({ error: '' })
    } catch (e) {
      if (e instanceof Error) {
        useImportExport.setState({ error: e.message })
      }
    }
  },
  fireConvert: async (mode, geometry) => {
    if (mode === 'Export') {
      await get().exportConvert()
    } else {
      await get().importConvert(geometry)
    }
  },
  updateStats: (writeCode) => {
    const { feature, code } = get()
    let max = 0
    let total = 0
    let count = 0
    if (feature.type === 'Feature') {
      if (feature.geometry.type === 'MultiPoint') {
        const { coordinates } = feature.geometry
        coordinates.forEach((point, j) => {
          const next = j ? coordinates[j + 1] : coordinates.at(-1)
          if (next) {
            const dis = distance(point, next, { units: 'meters' })
            if (dis > max) max = dis
            total += dis
          }
          count++
        })
      }
    } else {
      feature.features.forEach((f) => {
        if (f.geometry.type === 'MultiPoint') {
          const { coordinates } = f.geometry
          coordinates.forEach((point, j) => {
            const next = j ? coordinates[j + 1] : coordinates.at(-1)
            if (next) {
              const dis = distance(point, next, { units: 'meters' })
              if (dis > max) max = dis
              total += dis
            }
            count++
          })
        }
      })
    }
    set({
      stats: {
        max,
        total,
        count,
      },
      code: writeCode ? JSON.stringify(feature, null, 2) : code,
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
