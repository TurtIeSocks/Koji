import create from 'zustand'
import type { AlertProps } from '@mui/material'

import type {
  AdminProject,
  KojiStats,
  FeatureCollection,
  StoreNoFn,
  KojiTileServer,
} from '@assets/types'
import { collectionToObject } from '@services/utils'
import { ALL_FENCES, ALL_ROUTES } from '@assets/constants'

type CacheKey = StoreNoFn<UseStatic>

export interface UseStatic {
  networkStatus: {
    message: string
    status: number
    severity: AlertProps['severity']
  }
  bounds: {
    min_lat: number
    max_lat: number
    min_lon: number
    max_lon: number
  }
  loading: Record<string, KojiStats | null | false>
  loadingAbort: Record<string, AbortController | null>
  updateButton: boolean
  totalLoadingTime: number
  selected: string[]
  tileServers: KojiTileServer[]
  kojiRoutes: { name: string; id: number; type: string }[]
  scannerRoutes: { name: string; id: number; type: string }[]
  scannerType: string
  dangerous: boolean
  geojson: FeatureCollection
  layerEditing: {
    cutMode: boolean
    editMode: boolean
    rotateMode: boolean
    dragMode: boolean
    drawMode: boolean
    removalMode: boolean
  }
  // isEditing: () => boolean
  dialogs: {
    convert: boolean
    manager: boolean
    keyboard: boolean
  }
  forceRedraw: boolean
  forceFetch: boolean
  importWizard: {
    open: boolean
    nameProp: string
    props: string[]
    customName: string
    modifier: 'capitalize' | 'lowercase' | 'uppercase' | 'none'
    allProjects: number[]
    allGeofences: number | string
    allFenceMode: '' | typeof ALL_FENCES
    allRouteMode: '' | typeof ALL_ROUTES
    checked: Record<string, boolean>
  }
  projects: Record<number | string, AdminProject>
  clickedLocation: [number, number]
  combinePolyMode: boolean
  setStatic: <T extends CacheKey>(
    key: T,
    init: UseStatic[T] | ((prev: UseStatic[T]) => UseStatic[T]),
  ) => void
  setGeojson: (
    newGeojson: FeatureCollection,
    noSet?: boolean,
  ) => FeatureCollection
}

export const useStatic = create<UseStatic>((set, get) => ({
  networkStatus: {
    message: '',
    status: 0,
    severity: 'info',
  },
  bounds: {
    min_lat: 0,
    max_lat: 0,
    min_lon: 0,
    max_lon: 0,
  },
  loading: {},
  loadingAbort: {},
  totalLoadingTime: 0,
  updateButton: false,
  selected: [],
  scannerRoutes: [],
  kojiRoutes: [],
  tileServers: [],
  instances: {},
  geofences: {},
  routes: [],
  scannerType: 'rdm',
  dangerous: false,
  geojson: {
    type: 'FeatureCollection',
    features: [],
  },
  layerEditing: {
    cutMode: false,
    dragMode: false,
    drawMode: false,
    editMode: false,
    removalMode: false,
    rotateMode: false,
  },
  // isEditing: () => Object.values(get().layerEditing).some((v) => v),
  forceRedraw: false,
  forceFetch: false,
  importWizard: {
    open: false,
    nameProp: '',
    props: [],
    customName: '',
    modifier: 'none',
    allProjects: [],
    allGeofences: '',
    allFenceMode: '',
    allRouteMode: '',
    checked: {},
  },
  dialogs: {
    convert: false,
    manager: false,
    keyboard: false,
  },
  projects: {},
  clickedLocation: [0, 0],
  combinePolyMode: false,
  setStatic: (key, newValue) => {
    set((state) => ({
      [key]: typeof newValue === 'function' ? newValue(state[key]) : newValue,
    }))
  },
  setGeojson: (newGeojson, noSet) => {
    const { geojson } = get()
    const updated: FeatureCollection = {
      ...geojson,
      features: Object.values({
        ...collectionToObject(geojson),
        ...collectionToObject(newGeojson),
      }),
    }
    if (!noSet) {
      set({
        geojson: updated,
      })
    }
    return updated
  },
}))
