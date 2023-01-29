/* eslint-disable @typescript-eslint/ban-types */
import create from 'zustand'
import type {
  OnlyType,
  DbOption,
  KojiKey,
  Feature,
  Category,
} from '@assets/types'
import { ALL_FENCES, ALL_ROUTES } from '@assets/constants'

type CacheKey = keyof OnlyType<UseDbCache, Function, false>

export interface UseDbCache {
  route: Record<string | number, DbOption>
  project: Record<string | number, DbOption>
  geofence: Record<string | number, DbOption>
  scanner: Record<KojiKey, DbOption>
  feature: Record<KojiKey, Feature>
  getOptions: (
    ...args: Exclude<CacheKey, 'feature'>[]
  ) => Record<KojiKey, DbOption>
  getRecord: <T extends CacheKey>(
    key: T,
    id: keyof UseDbCache[T] | KojiKey,
  ) => UseDbCache[T][keyof UseDbCache[T]]
  getFromKojiKey: (key?: KojiKey | string) => DbOption | null
  getRecords: <T extends CacheKey>(key: T) => UseDbCache[T]
  getRouteByCategory: (category: Category, name?: string) => DbOption | null
  setRecord: <T extends CacheKey>(
    key: T,
    id: keyof UseDbCache[T],
    newValue: UseDbCache[T][keyof UseDbCache[T]],
  ) => void
  setRecords: <T extends CacheKey>(key: T, newValue: UseDbCache[T]) => void
  setDbCache: <T extends CacheKey>(
    key: T,
    init: UseDbCache[T] | ((prev: UseDbCache[T]) => UseDbCache[T]),
  ) => void
}

export const useDbCache = create<UseDbCache>((set, get) => ({
  route: {},
  project: {},
  geofence: {},
  scanner: {},
  feature: {},
  getOptions: (...args) => {
    const state = get()
    const returnObj: Record<KojiKey, DbOption> = {}
    args.forEach((key) => {
      Object.values(state[key]).forEach((value) => {
        returnObj[
          `${value.id}__${value.mode}__${
            key === 'scanner' ? 'SCANNER' : 'KOJI'
          }`
        ] = value
      })
    })
    return returnObj
  },
  getRouteByCategory: (category, name) => {
    const { route } = get()
    switch (category) {
      case 'gym':
        return (
          Object.values(route).find(
            (r) => r.name === name && r.mode.includes('Raid'),
          ) || null
        )
      case 'pokestop':
        return (
          Object.values(route).find(
            (r) => r.name === name && r.mode.includes('Quest'),
          ) || null
        )
      case 'spawnpoint':
        return (
          Object.values(route).find(
            (r) => r.name === name && r.mode.includes('Pokemon'),
          ) || null
        )
      default:
        return null
    }
  },
  getFromKojiKey: (key) => {
    if (!key) return null
    const [id, mode, source] = key.split('__')

    if (source === 'SCANNER') {
      return get().scanner[key as KojiKey] || null
    }
    if (ALL_FENCES.includes(mode as typeof ALL_FENCES[number])) {
      return get().geofence[id] || null
    }
    if (ALL_ROUTES.includes(mode as typeof ALL_ROUTES[number])) {
      return get().route[id] || null
    }
    return (
      get().getOptions('route', 'project', 'geofence')[key as KojiKey] || null
    )
  },
  getRecord: (key, id) => get()[key][id],
  getRecords: (key) => get()[key],
  setRecord: (key, id, newValue) =>
    set((state) => ({ [key]: { ...state[key], [id]: newValue } })),
  setRecords: (key, newValue) => set({ [key]: newValue }),
  setDbCache: (key, newValue) =>
    set((state) => ({
      [key]: typeof newValue === 'function' ? newValue(state[key]) : newValue,
    })),
}))
