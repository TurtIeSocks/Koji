import '@geoman-io/leaflet-geoman-free'
import '@geoman-io/leaflet-geoman-free/dist/leaflet-geoman.css'
import { useState } from 'react'
import useDeepCompareEffect from 'use-deep-compare-effect'
import * as L from 'leaflet'
import type { Feature } from 'geojson'

import type { GeomanProps } from './types'
import useEvents from './events/useEvents'
import layerEvents from './events/layers'

declare module 'leaflet' {
  interface Layer {
    pm: {
      enable: () => void
      enabled: () => boolean
    }
    feature: Feature
    getLatLng: () => L.LatLng
  }
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace PM {
    interface PMMap {
      getGeomanLayers(asFeatureGroup: true): L.FeatureGroup
      getGeomanLayers(asFeatureGroup?: false): L.Layer[]

      getGeomanDrawLayers(asFeatureGroup: true): L.FeatureGroup
      getGeomanDrawLayers(asFeatureGroup?: false): L.Layer[]
    }
    interface GlobalOptions {
      pmIgnore: boolean
    }
  }
}

export default function GeomanControls({
  map,
  options = {},
  globalOptions = { pmIgnore: false },
  pathOptions = {},
  lang = 'en',
  fallback = () => {},
  geojson = { type: 'FeatureCollection', features: [] },
  ...handlers
}: GeomanProps) {
  const [mounted, setMounted] = useState(false)

  useDeepCompareEffect(() => {
    map.pm.getGeomanLayers(true).eachLayer((layer) => {
      layer.removeFrom(map)
    })
    new L.GeoJSON(geojson).eachLayer((layer) => {
      L.PM.reInitLayer(layer)
      layerEvents(layer, { fallback, ...handlers }, 'on')
      if (layer.feature?.properties?.radius) {
        new L.Circle(layer.getLatLng(), {
          radius: layer.feature?.properties.radius,
        }).addTo(map)
      } else {
        map.addLayer(layer)
      }
    })
  }, [geojson])

  useDeepCompareEffect(() => {
    if (!map.pm.controlsVisible()) {
      map.pm.addControls(options)
      map.pm.setPathOptions(pathOptions)
      map.pm.setGlobalOptions(globalOptions)
      map.pm.setLang(lang)
      setMounted(true)
    }
    return () => {
      map.pm.removeControls()
      setMounted(false)
    }
  }, [options, globalOptions, pathOptions, lang])

  useEvents(mounted, { fallback, ...handlers }, map)

  return null
}
