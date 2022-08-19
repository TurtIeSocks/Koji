import create from 'zustand'

export interface UseStatic {
  instances: string[]
  scannerType: string
  tileServer: string
  setSettings: <T extends keyof UseStatic>(key: T, value: UseStatic[T]) => void
}

export const useStatic = create<UseStatic>((set) => ({
  instances: [],
  scannerType: 'rdm',
  tileServer:
    'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
  setSettings: (key, value) => set({ [key]: value }),
}))
