import '@geoman-io/leaflet-geoman-free'
import '@geoman-io/leaflet-geoman-free/dist/leaflet-geoman.css'
import { useState } from 'react'
import { useLeafletContext } from '@react-leaflet/core'
import useDeepCompareEffect from 'use-deep-compare-effect'
import * as L from 'leaflet'
import useEvents from './events/useEvents'

import type { GeomanProps } from './types'
import layerEvents from './events/layers'

declare module 'leaflet' {
  interface Layer {
    pm: {
      enable: () => void
      enabled: () => boolean
    }
  }
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace PM {
    interface PMMap {
      getGeomanLayers(asFeatureGroup?: true): L.FeatureGroup
      getGeomanLayers(asFeatureGroup?: false): L.Layer[]
      getGeomanLayers(asFeatureGroup?: boolean): L.FeatureGroup | L.Layer[]
    }
    interface GlobalOptions {
      pmIgnore: boolean
    }
  }
}

export default function GeomanControls({
  options = {},
  globalOptions = { pmIgnore: false },
  pathOptions = {},
  lang = 'en',
  fallback = () => {},
  geojson = { type: 'FeatureCollection', features: [] },
  ...handlers
}: GeomanProps) {
  const [mounted, setMounted] = useState(false)
  const { map } = useLeafletContext()

  useDeepCompareEffect(() => {
    map.pm.getGeomanLayers(true).eachLayer((layer) => {
      layer.removeFrom(map)
    })
    new L.GeoJSON(geojson).eachLayer((layer) => {
      L.PM.reInitLayer(layer)
      layerEvents(layer, { fallback, ...handlers }, 'on')
      map.addLayer(layer)
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
