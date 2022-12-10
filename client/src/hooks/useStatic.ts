import create from 'zustand'
import type { Feature, FeatureCollection } from 'geojson'
import * as L from 'leaflet'

import type { PixiMarker } from '@assets/types'

export interface UseStatic {
  pokestops: PixiMarker[]
  gyms: PixiMarker[]
  spawnpoints: PixiMarker[]
  getMarkers: () => PixiMarker[]
  selected: string[]
  instances: { [name: string]: Feature }
  geofences: { [name: string]: Feature }
  scannerType: string
  tileServer: string
  geojson: FeatureCollection
  cutMode: boolean
  editMode: boolean
  rotateMode: boolean
  dragMode: boolean
  drawMode: boolean
  removalMode: boolean
  forceRedraw: boolean
  activeLayer: L.Polygon | null
  popupLocation: L.LatLng
  forceFetch: boolean
  setSelected: (incoming: string[], stateKey: 'geofences' | 'instances') => void
  setStatic: <
    T extends keyof Omit<
      UseStatic,
      'setStatic' | 'setSelected' | 'setStaticAlt'
    >,
  >(
    key: T,
    init: UseStatic[T] | ((prev: UseStatic[T]) => void),
  ) => void
}

export const useStatic = create<UseStatic>((set, get) => ({
  pokestops: [],
  gyms: [],
  spawnpoints: [],
  getMarkers: () => {
    const { pokestops, gyms, spawnpoints } = get()
    return [...pokestops, ...gyms, ...spawnpoints]
  },
  selected: [],
  instances: {},
  geofences: {},
  scannerType: 'rdm',
  geojson: {
    type: 'FeatureCollection',
    features: [],
  },
  tileServer:
    'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
  cutMode: false,
  dragMode: false,
  drawMode: false,
  editMode: false,
  removalMode: false,
  rotateMode: false,
  forceRedraw: false,
  activeLayer: null,
  popupLocation: new L.LatLng(0, 0),
  forceFetch: false,
  setStatic: (key, newValue) => {
    set((state) => ({
      [key]: typeof newValue === 'function' ? newValue(state[key]) : newValue,
    }))
  },
  setSelected: (selected, stateKey) => {
    set({
      selected,
      geojson: {
        type: 'FeatureCollection',
        features: selected.map((name) => get()[stateKey][name]),
      },
    })
  },
}))
