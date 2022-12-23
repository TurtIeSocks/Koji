import create from 'zustand'
import type { Feature, FeatureCollection } from 'geojson'

import type { PixiMarker } from '@assets/types'
import { collectionToObject } from '@services/utils'

export interface UseStatic {
  loggedIn: boolean
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
  lineStrings: FeatureCollection
  polygons: FeatureCollection
  circles: FeatureCollection
  layerEditing: {
    cutMode: boolean
    editMode: boolean
    rotateMode: boolean
    dragMode: boolean
    drawMode: boolean
    removalMode: boolean
  }
  forceRedraw: boolean
  forceFetch: boolean
  setSelected: (incoming: string[], stateKey: 'geofences' | 'instances') => void
  setStatic: <
    T extends keyof Omit<
      UseStatic,
      'setStatic' | 'setSelected' | 'setStaticAlt' | 'setGeojson'
    >,
  >(
    key: T,
    init: UseStatic[T] | ((prev: UseStatic[T]) => UseStatic[T]),
  ) => void
  setGeojson: (
    newGeojson: FeatureCollection,
    noSet?: boolean,
  ) => FeatureCollection
}

export const useStatic = create<UseStatic>((set, get) => ({
  loggedIn: false,
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
  lineStrings: {
    type: 'FeatureCollection',
    features: [],
  },
  circles: {
    type: 'FeatureCollection',
    features: [],
  },
  polygons: {
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
  forceRedraw: false,
  forceFetch: false,
  setStatic: (key, newValue) => {
    set((state) => ({
      [key]: typeof newValue === 'function' ? newValue(state[key]) : newValue,
    }))
  },
  setSelected: (selected, stateKey) => {
    const { geojson, instances, geofences } = get()
    const fences = { instances, geofences }
    set({
      selected,
      geojson: {
        ...geojson,
        features: [
          ...Object.values({
            // ...collectionToObject(geojson),
            ...collectionToObject({
              type: 'FeatureCollection',
              features: [
                ...selected.map((name) => fences[stateKey][name]),
                ...geojson.features.filter(
                  (feature) => feature.properties?.type === undefined,
                ),
              ],
            }),
          }),
          ...geojson.features.filter(
            (feature) => feature.properties?.type === undefined,
          ),
        ],
      },
    })
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
