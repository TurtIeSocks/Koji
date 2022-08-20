import create from 'zustand'
import type { FeatureCollection } from 'geojson'

import type { Instance } from '@assets/types'

export interface UseStatic {
  instances: { [name: string]: Instance }
  validInstances: Set<string>
  scannerType: string
  tileServer: string
  geojson: FeatureCollection
  setStatic: <T extends keyof UseStatic>(key: T, value: UseStatic[T]) => void
}

export const useStatic = create<UseStatic>((set) => ({
  instances: {},
  validInstances: new Set(),
  scannerType: 'rdm',
  geojson: { type: 'FeatureCollection', features: [] },
  tileServer:
    'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
  setStatic: (key, value) => set({ [key]: value }),
}))
