/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable @typescript-eslint/no-namespace */

import type { Feature, FeatureCollection } from 'geojson'
import * as L from 'leaflet'

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

// for some reason locate.control @types augmentation is broken
declare module 'leaflet' {
  namespace Control {
    class Locate extends Control {
      constructor(locateOptions?: LocateOptions)
      onAdd(map: Map): HTMLElement
      start(): void
      stop(): void
      stopFollowing(): void
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
      createButtonCallback?:
        | ((container: HTMLDivElement, options: LocateOptions) => void)
        | undefined
      onLocationError?:
        | ((event: ErrorEvent, control: Locate) => void)
        | undefined
      onLocationOutsideMapBounds?: ((control: Locate) => void) | undefined
      showPopup?: boolean | undefined
      strings?: StringsOptions | undefined
      locateOptions?: L.LocateOptions | undefined
    }
    interface StringsOptions {
      title?: string | undefined
      metersUnit?: string | undefined
      feetUnit?: string | undefined
      popup?: string | undefined
      outsideMapBoundsMsg?: string | undefined
    }
  }

  namespace control {
    /**
     * Creates a Leaflet.Locate control
     */
    function locate(options?: Control.LocateOptions): Control.Locate
  }
}
