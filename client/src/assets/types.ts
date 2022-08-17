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

export interface Point {
  type: 'Point'
  coordinates: [number, number]
}
export interface Line {
  type: 'LineString'
  coordinates: [number, number][]
}

export interface GeoJSON {
  type: string
  features: {
    type: string
    geometry: Line
    properties: {
      [key: string]: string | number
    }
  }[]
}
