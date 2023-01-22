import type {
  Feature,
  FeatureCollection,
  GeoJsonTypes,
  MultiPoint,
} from 'geojson'

import type { UsePersist } from '@hooks/usePersist'
import type { UseStatic } from '@hooks/useStatic'
import { TABS } from './constants'

export type SpecificValueType<T, U> = {
  [k in keyof T]: T[k] extends U ? k : never
}[keyof T]

export type OnlyType<T, U> = { [k in SpecificValueType<T, U>]: U }

export interface Data {
  gyms: PixiMarker[]
  pokestops: PixiMarker[]
  spawnpoints: PixiMarker[]
}

export interface PixiMarker {
  i: `${'p' | 'g' | 'v' | 'u'}${number}` & { [0]: 'p' | 'g' | 'v' | 'u' }
  p: [number, number]
}

export interface Instance {
  name: string
  type: string
  data: FeatureCollection
}

export interface KojiResponse<T = FeatureCollection> {
  data: T
  status_code: number
  status: string
  message: string
  stats?: {
    best_clusters: [number, number][]
    best_cluster_point_count: number
    cluster_time: number
    total_points: number
    points_covered: number
    total_clusters: number
    total_distance: number
    longest_distance: number
  }
}

export interface Config {
  start_lat: number
  start_lon: number
  tile_server: string
  scanner_type: string
  logged_in: boolean
  dangerous: boolean
}

export interface Circle {
  id: string
  lat: number
  lng: number
  radius: number
  type: 'circle'
}

export interface Polygon {
  id: string
  positions: [number, number][]
  type: 'polygon'
}

export type CombinedState = Partial<UsePersist> & Partial<UseStatic>

export type ObjectInput = { lat: number; lon: number }[]
export type MultiObjectInput = ObjectInput[]

export type ArrayInput = number[][]
export type MultiArrayInput = ArrayInput[]

export interface Poracle {
  name?: string
  id?: number
  color?: string
  path?: ArrayInput
  multipath?: MultiArrayInput
  group?: string
  description?: string
  user_selectable?: boolean
  display_in_matches?: boolean
}

export type ToConvert =
  | ObjectInput
  | MultiObjectInput
  | ArrayInput
  | MultiArrayInput
  | Feature
  | FeatureCollection
  | string
  | Poracle
  | Feature[]

export interface PopupProps {
  id: Feature['id']
  properties: Feature['properties']
}

export interface KojiGeofence {
  id: number
  name: string
  mode: string
  area: Feature
}

export interface ClientGeofence extends KojiGeofence {
  properties: { key: string; value: string | number | boolean }[]
  related: number[]
}

export interface KojiProject {
  id: number
  name: string
  created_at: Date | string
  updated_at: Date | string
  api_endpoint?: string
  api_key?: string
  scanner: boolean
}

export interface ClientProject extends KojiProject {
  related: number[]
}

export interface KojiRoute {
  id: number
  geofence_id: number
  name: string
  description?: string
  mode: string
  geometry: MultiPoint
}

export interface KojiStats {
  best_clusters: [number, number][]
  best_cluster_point_count: number
  cluster_time: number
  total_points: number
  points_covered: number
  total_clusters: number
  total_distance: number
  longest_distance: number
  fetch_time: number
}

export interface Option {
  id: number
  type: string
  name: string
  geoType?: Exclude<GeoJsonTypes, 'Feature' | 'FeatureCollection'>
}

export type TabOption = typeof TABS[number]
