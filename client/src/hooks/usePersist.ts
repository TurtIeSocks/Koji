import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { TabOption, Category, ConversionOptions } from '@assets/types'
import {
  GEOMETRY_CONVERSION_TYPES,
  S2_CELL_LEVELS,
  BOOTSTRAP_LEVELS,
} from '@assets/constants'

export interface UsePersist {
  // Drawing
  snappable: boolean
  continueDrawing: boolean
  setActiveMode: 'hover' | 'click'

  // Client Settings
  darkMode: boolean
  menuItem: TabOption
  tab: number
  drawer: boolean
  tileServer: string
  location: [number, number]
  zoom: number
  kbShortcuts: Record<string, string>
  polygonExportMode: ConversionOptions
  showRouteIndex: boolean
  geometryType: typeof GEOMETRY_CONVERSION_TYPES[number]
  loadingScreen: boolean
  simplifyPolygons: boolean
  scaleMarkers: boolean

  // Layers
  spawnpoint: boolean
  gym: boolean
  pokestop: boolean
  pokestopRange: boolean
  data: 'all' | 'area' | 'bound'
  last_seen: Date
  showCircles: boolean
  showLines: boolean
  showPolygons: boolean
  showArrows: boolean
  s2cells: number[]
  s2DisplayMode: 'all' | 'covered' | 'none'
  s2FillMode: 'all' | 'simple'

  // Clustering
  category: Category | 'fort'
  cluster_mode: 'Fast' | 'Balanced' | 'BruteForce'
  tth: 'All' | 'Known' | 'Unknown'
  lineColorRules: { distance: number; color: string }[]
  mode: 'bootstrap' | 'route' | 'cluster'
  sort_by: 'GeoHash' | 'Random' | 'ClusterCount'
  radius: number | ''
  min_points: number | ''
  only_unique: boolean
  route_split_level: number | ''
  cluster_split_level: number | ''
  save_to_db: boolean
  save_to_scanner: boolean
  skipRendering: boolean
  fast: boolean
  calculation_mode: 'Radius' | 'S2'
  s2_level: typeof S2_CELL_LEVELS[number]
  s2_size: typeof BOOTSTRAP_LEVELS[number]
  // generations: number | ''
  // routing_time: number | ''
  // devices: number | ''
  // route_chunk_size: number | ''

  // Dev
  nativeLeaflet: boolean
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
      cluster_mode: 'Fast',
      lineColorRules: [
        { distance: 500, color: '#197E13' },
        { distance: 1000, color: '#FFFF0C' },
        { distance: 1500, color: '#FEA71D' },
      ],
      scaleMarkers: false,
      tileServer:
        'https://{s}.basemaps.cartocdn.com/rastertiles/voyager_labels_under/{z}/{x}/{y}{r}.png',
      tth: 'All',
      spawnpoint: false,
      gym: true,
      pokestop: true,
      pokestopRange: false,
      kbShortcuts: {},
      mode: 'cluster',
      data: 'bound',
      sort_by: 'GeoHash',
      s2cells: [],
      s2DisplayMode: 'none',
      s2FillMode: 'simple',
      radius: 70,
      route_split_level: 1,
      cluster_split_level: 10,
      // routing_chunk_size: 0,
      calculation_mode: 'Radius',
      s2_level: 15,
      s2_size: 9,
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
      last_seen: (() => {
        const date = new Date()
        date.setMinutes(0)
        date.setSeconds(0)
        return date
      })(),
      snappable: true,
      continueDrawing: true,
      loadingScreen: true,
      simplifyPolygons: true,
      setActiveMode: 'hover',
      colorByGeohash: false,
      geohashPrecision: 6,
      setStore: (key, value) => set({ [key]: value }),
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
