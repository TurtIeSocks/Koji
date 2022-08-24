import * as React from 'react'
import { useMap } from 'react-leaflet'

import GeomanControls from '@components/geoman/src'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'

export default function Drawing2() {
  const radius = useStore((s) => s.radius)

  const setStatic = useStatic((s) => s.setStatic)
  const geojson = useStatic((s) => s.geojson)

  const map = useMap()

  const handleChange = () => {
    const newGeo = map.pm.getGeomanLayers(true).toGeoJSON()
    if (newGeo.type === 'FeatureCollection') {
      const withRadius = {
        ...newGeo,
        features: newGeo.features.map((feature) => ({
          ...feature,
          properties: {
            ...feature.properties,
            radius: feature.geometry.type === 'Point' ? radius : undefined,
          },
        })),
      }
      setStatic('geojson', withRadius)
      setStatic(
        'selected',
        withRadius.features.map((x) => x.properties?.name).filter(Boolean),
      )
    }
  }

  React.useEffect(() => {
    handleChange()
  }, [radius])

  return (
    <GeomanControls
      key={radius}
      map={map}
      options={{
        position: 'topright',
        drawMarker: false,
        drawRectangle: false,
        drawText: false,
        drawPolyline: false,
        // Going to swap these two once a PR is merged
        drawCircle: false,
        drawCircleMarker: true,
      }}
      globalOptions={{
        pmIgnore: false,
        snappable: true,
        continueDrawing: true,
        // Also waiting for a PR to be merged to enable this...
        // templineStyle: { radius: radius || 70 },
        editable: false,
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
