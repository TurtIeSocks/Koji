export interface Data {
  gyms: PixiMarker[]
  pokestops: PixiMarker[]
  spawnpoints: PixiMarker[]
}

export interface PixiMarker {
  id: string
  iconId: 'p' | 'g' | 'v' | 'u'
  position: [number, number]
}

export interface Instance {
  name: string
  type_: string
  data: string
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
