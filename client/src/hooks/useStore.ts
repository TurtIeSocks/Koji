import create from 'zustand'
import { persist } from 'zustand/middleware'
import type { FeatureCollection } from 'geojson'

export interface UseStore {
  drawer: boolean
  setDrawer: (drawer: boolean) => void
  location: [number, number]
  setLocation: (location: UseStore['location']) => void
  zoom: number
  setZoom: (zoom: UseStore['zoom']) => void
  category: 'pokestop' | 'gym' | 'spawnpoint'
  spawnpoint: boolean
  gym: boolean
  pokestop: boolean
  data: 'all' | 'area' | 'bound'
  mode: 'bootstrap' | 'route' | 'cluster'
  instance: string
  radius: number | ''
  generations: number | ''
  showCircles: boolean
  showLines: boolean
  showPolygon: boolean
  renderer: 'performance' | 'quality'
  devices: number | ''
  export: {
    total: number
    max: number
    route: [number, number][][]
  }
  geojson: FeatureCollection
  setSettings: <T extends keyof UseStore>(key: T, value: UseStore[T]) => void
}

export const useStore = create(
  persist<UseStore>(
    (set) => ({
      drawer: false,
      setDrawer: (drawer) => set({ drawer }),
      location: [0, 0],
      setLocation: (location) => set({ location }),
      zoom: 16,
      setZoom: (zoom) => set({ zoom }),
      category: 'pokestop',
      spawnpoint: true,
      gym: true,
      pokestop: true,
      mode: 'cluster',
      data: 'all',
      instance: '',
      radius: 70,
      generations: 100,
      showCircles: true,
      showLines: true,
      showPolygon: true,
      renderer: 'performance',
      devices: 1,
      export: {
        total: 0,
        max: 0,
        route: [],
      },
      geojson: { type: 'FeatureCollection', features: [] },
      setSettings: (key, value) => set({ [key]: value }),
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
