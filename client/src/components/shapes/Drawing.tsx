import * as React from 'react'
import * as L from 'leaflet'
import { FeatureGroup } from 'react-leaflet'
import { EditControl } from 'react-leaflet-draw'
import { useStore } from '@hooks/useStore'

export default function Drawing() {
  const geojson = useStore((s) => s.geojson)
  const setSettings = useStore((s) => s.setSettings)
  const [ref, setRef] = React.useState<L.FeatureGroup | null>(null)

  const onFeatureGroupReady = (reactFGref: L.FeatureGroup | null) => {
    new L.GeoJSON().eachLayer((layer) => {
      if (reactFGref) reactFGref.addLayer(layer)
    })
    setRef(reactFGref)
  }

  const handleChange = () => {
    const geo = ref?.toGeoJSON()
    if (geo?.type === 'FeatureCollection') {
      setSettings('geojson', geo)
    }
  }

  return (
    <FeatureGroup
      ref={(newRef) => {
        onFeatureGroupReady(newRef)
      }}
    >
      <EditControl
        position="topright"
        onEdited={handleChange}
        onCreated={handleChange}
        onDeleted={handleChange}
        onMounted={() => {
          if (geojson.features.length) {
            new L.GeoJSON(geojson).eachLayer((layer) => {
              if (ref) ref.addLayer(layer)
            })
          }
        }}
        draw={{
          rectangle: false,
          circle: false,
          polyline: false,
          polygon: true,
          marker: false,
          circlemarker: false,
        }}
      />
    </FeatureGroup>
  )
}
