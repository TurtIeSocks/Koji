/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable @typescript-eslint/no-namespace */
import L from 'leaflet'

declare module 'leaflet' {
  namespace Control {
    class Locate extends Control {
      constructor(locateOptions?: LocateOptions)
      onAdd(map: Map): HTMLElement
      start(): void
      stop(): void
      setView(): void
    }
    interface LocateOptions {
      position?: string | undefined
      layer?: Layer | undefined
      setView?: boolean | string | undefined
      keepCurrentZoomLevel?: boolean | undefined
      initialZoomLevel?: number | boolean | undefined
      flyTo?: boolean | undefined
      clickBehavior?: any
      returnToPrevBounds?: boolean | undefined
      cacheLocation?: boolean | undefined
      drawCircle?: boolean | undefined
      drawMarker?: boolean | undefined
      showCompass?: boolean | undefined
      markerClass?: any
      compassClass?: any
      circleStyle?: PathOptions | undefined
      markerStyle?: PathOptions | MarkerOptions | undefined
      compassStyle?: PathOptions | undefined
      followCircleStyle?: PathOptions | undefined
      followMarkerStyle?: PathOptions | undefined
      icon?: string | undefined
      iconLoading?: string | undefined
      iconElementTag?: string | undefined
      textElementTag?: string | undefined
      circlePadding?: number[] | undefined
      metric?: boolean | undefined
      createButtonCallback?: any
      onLocationError?: any
      onLocationOutsideMapBounds?: any
      showPopup?: boolean | undefined
      strings?: any
      locateOptions?: L.LocateOptions | undefined
    }
  }
  namespace control {
    /**
     * Creates a Leaflet.Locate control
     */
    function locate(options?: Control.LocateOptions): Control.Locate
  }
}

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
