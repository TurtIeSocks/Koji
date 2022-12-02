import create from 'zustand'
import { persist } from 'zustand/middleware'
import type { FeatureCollection } from 'geojson'

export interface UseStore {
  tab: number
  drawer: boolean
  location: [number, number]
  zoom: number
  category: 'pokestop' | 'gym' | 'spawnpoint'
  spawnpoint: boolean
  gym: boolean
  pokestop: boolean
  data: 'all' | 'area' | 'bound'
  mode: 'bootstrap' | 'route' | 'cluster'
  radius: number | ''
  min_points: number | ''
  generations: number | ''
  routing_time: number | ''
  showCircles: boolean
  showLines: boolean
  showPolygon: boolean
  nativeLeaflet: boolean
  devices: number | ''
  geojson: FeatureCollection
  polygonExportMode:
    | 'feature'
    | 'featureCollection'
    | 'array'
    | 'struct'
    | 'text'
    | 'altText'
  export: {
    total: number
    max: number
    route: [number, number][][]
  }
  snappable: boolean
  continueDrawing: boolean
  fast: boolean
  autoMode: boolean
  setStore: <T extends keyof UseStore>(key: T, value: UseStore[T]) => void
}

export const useStore = create(
  persist<UseStore>(
    (set) => ({
      tab: 0,
      drawer: false,
      location: [0, 0],
      zoom: 16,
      category: 'pokestop',
      spawnpoint: true,
      gym: true,
      pokestop: true,
      mode: 'cluster',
      data: 'all',
      radius: 70,
      generations: 1,
      routing_time: 1,
      min_points: 3,
      showCircles: true,
      showLines: true,
      showPolygon: true,
      nativeLeaflet: false,
      devices: 1,
      polygonExportMode: 'feature',
      fast: false,
      export: {
        total: 0,
        max: 0,
        route: [],
      },
      geojson: { type: 'FeatureCollection', features: [] },
      setStore: (key, value) => set({ [key]: value }),
      snappable: true,
      continueDrawing: true,
      autoMode: false,
    }),
    {
      name: 'local',
      partialize: (state) =>
        Object.fromEntries(
          Object.entries(state).filter(([key]) => !['export'].includes(key)),
        ) as UseStore,
    },
  ),
)
