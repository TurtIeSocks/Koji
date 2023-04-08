import { create } from 'zustand'

export interface UseRaStore {
  bulkAssignProject: boolean
  bulkAssignGeofence: boolean
  geofenceCreateDialog: boolean
  bulkAssignParent: boolean
  setRaStore: <T extends keyof Omit<UseRaStore, 'setRaStore'>>(
    key: T,
    init: UseRaStore[T] | ((prev: UseRaStore[T]) => UseRaStore[T]),
  ) => void
}

export const useRaStore = create<UseRaStore>((set) => ({
  bulkAssignProject: false,
  bulkAssignGeofence: false,
  bulkAssignParent: false,
  geofenceCreateDialog: false,
  setRaStore: (key, newValue) => {
    set((state) => ({
      [key]: typeof newValue === 'function' ? newValue(state[key]) : newValue,
    }))
  },
}))
