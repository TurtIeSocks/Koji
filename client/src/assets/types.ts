import type { Feature, FeatureCollection } from 'geojson'

import type { UseStore } from '@hooks/useStore'
import type { UseStatic } from '@hooks/useStatic'

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

export interface Config {
  start_lat: number
  start_lon: number
  tile_server: string
  scanner_type: string
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

export type Shape = Circle | Polygon

export type CombinedState = Partial<UseStore> & Partial<UseStatic>

export type ObjectInput = { lat: number; lon: number }[]
export type MultiObjectInput = ObjectInput[]

export type ArrayInput = number[][]
export type MultiArrayInput = ArrayInput[]

export type ToConvert =
  | ObjectInput
  | MultiObjectInput
  | ArrayInput
  | MultiArrayInput
  | Feature
  | FeatureCollection
  | string
