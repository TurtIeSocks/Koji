import create from 'zustand'
import { persist } from 'zustand/middleware'
import type {
  TabOption,
  FeatureCollection,
  Category,
  ConversionOptions,
} from '@assets/types'
import { GEOMETRY_CONVERSION_TYPES } from '@assets/constants'

export interface UsePersist {
  darkMode: boolean
  tab: number
  drawer: boolean
  location: [number, number]
  zoom: number
  category: Category
  tileServer: string
  tth: 'All' | 'Known' | 'Unknown'
  spawnpoint: boolean
  gym: boolean
  pokestop: boolean
  pokestopRange: boolean
  data: 'all' | 'area' | 'bound'
  mode: 'bootstrap' | 'route' | 'cluster'
  sort_by: 'GeoHash' | 'Random' | 'ClusterCount'
  radius: number | ''
  min_points: number | ''
  only_unique: boolean
  route_split_level: number | ''
  // generations: number | ''
  // routing_time: number | ''
  showCircles: boolean
  showLines: boolean
  showPolygons: boolean
  showArrows: boolean
  nativeLeaflet: boolean
  last_seen: Date
  // devices: number | ''
  geojson: FeatureCollection
  // route_chunk_size: number | ''
  kbShortcuts: Record<string, string>
  polygonExportMode: ConversionOptions
  showRouteIndex: boolean
  geometryType: typeof GEOMETRY_CONVERSION_TYPES[number]
  menuItem: TabOption
  export: {
    total: number
    max: number
    route: [number, number][][]
  }
  s2cells: number[]
  save_to_db: boolean
  save_to_scanner: boolean
  skipRendering: boolean
  snappable: boolean
  continueDrawing: boolean
  fast: boolean
  loadingScreen: boolean
  simplifyPolygons: boolean
  setActiveMode: 'hover' | 'click'
  colorByGeohash: boolean
  geohashPrecision: number
  setStore: <T extends keyof UsePersist>(key: T, value: UsePersist[T]) => void
}

export const usePersist = create(
  persist<UsePersist>(
    (set) => ({
      darkMode: true,
      tab: 0,
      drawer: false,
      location: [0, 0],
      zoom: 18,
      category: 'pokestop',
      tileServer:
        'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
      tth: 'All',
      spawnpoint: false,
      gym: true,
      pokestop: true,
      pokestopRange: false,
      kbShortcuts: {
        draw: 'ctrl+d',
        move: 'ctrl+m',
        erase: 'ctrl+e',
        rectangle: 'ctrl+r',
        circle: 'ctrl+c',
        polygon: 'ctrl+p',
      },
      mode: 'cluster',
      data: 'bound',
      sort_by: 'GeoHash',
      s2cells: [],
      radius: 70,
      route_split_level: 1,
      routing_chunk_size: 0,
      min_points: 3,
      only_unique: false,
      save_to_db: false,
      save_to_scanner: false,
      skipRendering: false,
      showCircles: true,
      showLines: true,
      showPolygons: true,
      showArrows: true,
      nativeLeaflet: false,
      polygonExportMode: 'feature',
      showRouteIndex: false,
      geometryType: 'Polygon',
      menuItem: 'Manage',
      fast: false,
      export: {
        total: 0,
        max: 0,
        route: [],
      },
      last_seen: (() => {
        const date = new Date()
        date.setMinutes(0)
        date.setSeconds(0)
        return date
      })(),
      geojson: { type: 'FeatureCollection', features: [] },
      setStore: (key, value) => set({ [key]: value }),
      snappable: true,
      continueDrawing: true,
      loadingScreen: true,
      simplifyPolygons: true,
      setActiveMode: 'hover',
      colorByGeohash: false,
      geohashPrecision: 6,
    }),
    {
      name: 'local',
      partialize: (state) =>
        Object.fromEntries(
          Object.entries(state).filter(([key]) => !['export'].includes(key)),
        ) as UsePersist,
    },
  ),
)
