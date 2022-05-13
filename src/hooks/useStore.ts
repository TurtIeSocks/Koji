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
      location: [0, 0],
      setLocation: (location) => set({ location }),
      zoom: 18,
      setZoom: (zoom) => set({ zoom }),
    }),
    {
      name: 'local',
    },
  ),
)
