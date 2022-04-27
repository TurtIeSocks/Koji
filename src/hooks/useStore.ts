import create from 'zustand'
import { persist } from 'zustand/middleware'

export interface UseStore {
  location: [number, number]
  setLocation: (location: UseStore['location']) => void
  zoom: number
  setZoom: (zoom: UseStore['zoom']) => void
}

export const useStore = create(
  persist<UseStore>(
    (set) => ({
      location: [+inject.START_LAT || 0, +inject.START_LON || 0],
      setLocation: (location) => set({ location }),
      zoom: 18,
      setZoom: (zoom) => set({ zoom }),
    }),
    {
      name: 'local',
    },
  ),
)
