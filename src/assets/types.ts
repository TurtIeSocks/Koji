export interface Data {
  gyms: PixiMarker[]
  pokestops: PixiMarker[]
  spawnpoints: PixiMarker[]
}

export interface PixiMarker {
  id: string
  iconId: 'pokestop' | 'gym' | 'spawnpoint_true' | 'spawnpoint_false'
  position: [number, number]
}