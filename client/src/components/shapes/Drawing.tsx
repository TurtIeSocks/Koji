import * as React from 'react'
import { useMap } from 'react-leaflet'

import GeomanControl from '@components/geoman/src'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'

export default function Drawing2() {
  const setStatic = useStatic((s) => s.setStatic)
  const setStore = useStore((s) => s.setStore)
  const geojson = useStatic((s) => s.geojson)

  const map = useMap()

  const handleChange = () => {
    const newGeo = map.pm.getGeomanLayers(true).toGeoJSON()
    if (newGeo.type === 'FeatureCollection') {
      setStatic('geojson', newGeo)
      setStore('geojson', newGeo)
      setStatic(
        'selected',
        newGeo.features.map((x) => x.properties?.name).filter(Boolean),
      )
    }
  }

  return (
    <GeomanControl
      options={{
        position: 'topright',
        drawMarker: false,
        drawRectangle: false,
        drawText: false,
        drawPolyline: false,
        drawCircle: false,
        drawCircleMarker: false,
      }}
      globalOptions={{
        pmIgnore: false,
        snappable: true,
        continueDrawing: true,
      }}
      geojson={geojson}
      onCreate={handleChange}
      onUpdate={handleChange}
      onMapRemove={handleChange}
      onDragEnd={handleChange}
      onMapCut={handleChange}
    />
  )
}
