import create from 'zustand'
import type { AlertProps } from '@mui/material'

import type {
  AdminProject,
  KojiStats,
  FeatureCollection,
  StoreNoFn,
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
  loading: Record<string, KojiStats | null | false>
  updateButton: boolean
  totalLoadingTime: number
  selected: string[]
  kojiRoutes: { name: string; id: number; type: string }[]
  scannerRoutes: { name: string; id: number; type: string }[]
  scannerType: string
  dangerous: boolean
  tileServer: string
  geojson: FeatureCollection
  layerEditing: {
    cutMode: boolean
    editMode: boolean
    rotateMode: boolean
    dragMode: boolean
    drawMode: boolean
    removalMode: boolean
  }
  isEditing: () => boolean
  dialogs: {
    convert: boolean
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
  loading: {},
  totalLoadingTime: 0,
  updateButton: false,
  selected: [],
  scannerRoutes: [],
  kojiRoutes: [],
  instances: {},
  geofences: {},
  routes: [],
  scannerType: 'rdm',
  dangerous: false,
  geojson: {
    type: 'FeatureCollection',
    features: [],
  },
  tileServer:
    'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
  layerEditing: {
    cutMode: false,
    dragMode: false,
    drawMode: false,
    editMode: false,
    removalMode: false,
    rotateMode: false,
  },
  isEditing: () => Object.values(get().layerEditing).some((v) => v),
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
