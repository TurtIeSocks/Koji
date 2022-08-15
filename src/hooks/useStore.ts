import create from 'zustand'
import { persist } from 'zustand/middleware'

export interface UseStore {
  drawer: boolean
  setDrawer: (drawer: boolean) => void
  location: [number, number]
  setLocation: (location: UseStore['location']) => void
  zoom: number
  setZoom: (zoom: UseStore['zoom']) => void
  apiSettings: {
    instance: string
    radius: number
    generations: number
    mode: 'bootstrap' | 'route' | 'cluster'
  }
  setApiSettings: (instanceForm: UseStore['apiSettings']) => void
}

export const useStore = create(
  persist<UseStore>(
    (set) => ({
      drawer: false,
      setDrawer: (drawer) => set({ drawer }),
      location: [0, 0],
      setLocation: (location) => set({ location }),
      zoom: 18,
      setZoom: (zoom) => set({ zoom }),
      apiSettings: {
        instance: '',
        radius: 70,
        generations: 100,
        mode: 'cluster',
      },
      setApiSettings: (instanceForm) => set({ apiSettings: instanceForm }),
    }),
    {
      name: 'local',
    },
  ),
)
