import create from 'zustand'
import { persist } from 'zustand/middleware'

export interface UseStore {
  location: [number, number]
  setLocation: (location: UseStore['location']) => void
  zoom: number
  setZoom: (zoom: UseStore['zoom']) => void
  instanceForm: { name: string; radius: number; generations: number }
  setInstanceForm: (instanceForm: UseStore['instanceForm']) => void
}

export const useStore = create(
  persist<UseStore>(
    (set) => ({
      location: [0, 0],
      setLocation: (location) => set({ location }),
      zoom: 18,
      setZoom: (zoom) => set({ zoom }),
      instanceForm: { name: '', radius: 70, generations: 100 },
      setInstanceForm: (instanceForm) => set({ instanceForm }),
    }),
    {
      name: 'local',
    },
  ),
)

export interface UseStatic {
  open: string
  setOpen: (open: UseStatic['open']) => void
  handleClose: () => void
}

export const useStatic = create<UseStatic>((set) => ({
  open: '',
  setOpen: (open) => set({ open }),
  handleClose: () => {
    window.location.hash = ''
    set({ open: '' })
  },
}))
