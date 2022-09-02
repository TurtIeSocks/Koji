import * as React from 'react'
import { FeatureGroup, useMapEvents } from 'react-leaflet'
import useDeepCompareEffect from 'use-deep-compare-effect'
import * as L from 'leaflet'
import type { FeatureCollection } from 'geojson'

import { GeomanControls } from 'react-leaflet-geoman-v2'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'

export default function Drawing() {
  const radius = useStore((s) => s.radius)
  const snappable = useStore((s) => s.snappable)
  const continueDrawing = useStore((s) => s.continueDrawing)

  const setStatic = useStatic((s) => s.setStatic)
  const geojson = useStatic((s) => s.geojson)
  const forceRedraw = useStatic((s) => s.forceRedraw)
  const map = useMapEvents({
    click: (e) => setStatic('popupLocation', e.latlng),
    popupclose: () => setStatic('activeLayer', null),
  })

  const ref = React.useRef<L.FeatureGroup>(null)

  const handleChange = () => {
    const newGeo: FeatureCollection = {
      type: 'FeatureCollection',
      features: [],
    }
    const layers = ref.current?.getLayers()
    if (layers) {
      layers.forEach((layer) => {
        if (layer instanceof L.Circle || layer instanceof L.CircleMarker) {
          const { lat, lng } = layer.getLatLng()
          newGeo.features.push({
            type: 'Feature',
            properties: {
              radius,
            },
            geometry: {
              type: 'Point',
              coordinates: [lng, lat],
            },
          })
        } else if (
          layer instanceof L.Marker ||
          layer instanceof L.Polygon ||
          layer instanceof L.Rectangle ||
          layer instanceof L.Polyline
        ) {
          if (layer instanceof L.Polygon) {
            layer.on('click', () => setStatic('activeLayer', layer))
          }
          newGeo.features.push(layer.toGeoJSON())
        }
      })
    }
    setStatic('geojson', newGeo)
    setStatic(
      'selected',
      newGeo.features.map((x) => x.properties?.name).filter(Boolean),
    )
  }

  useDeepCompareEffect(() => {
    if (ref.current) {
      ref.current.eachLayer((layer) => {
        if (ref.current) layer.removeFrom(ref.current as unknown as L.Map)
      })
    }
    L.geoJSON(geojson).eachLayer((layer) => {
      if (
        layer instanceof L.Polyline ||
        layer instanceof L.Polygon ||
        layer instanceof L.Marker
      ) {
        if (layer?.feature?.properties.radius && ref.current) {
          new L.Circle(layer.feature.geometry.coordinates.slice().reverse(), {
            radius: radius || layer.feature?.properties.radius,
          }).addTo(ref.current)
        } else if (layer instanceof L.Polygon) {
          ref.current?.addLayer(
            layer.on('click', () => setStatic('activeLayer', layer)),
          )
        } else {
          ref.current?.addLayer(layer)
        }
      }
    })
  }, [geojson, radius])

  return (
    <FeatureGroup ref={ref}>
      <GeomanControls
        options={{
          position: 'topright',
          drawText: false,
          drawMarker: false,
          // swap these two when leaflet-geoman PR is merged
          drawCircleMarker: true,
          drawCircle: false,
          drawRectangle: false,
          drawPolyline: false,
          drawPolygon: true,
        }}
        globalOptions={{
          continueDrawing,
          snappable,
          editable: false,
        }}
        onCreate={handleChange}
        onChange={handleChange}
        onUpdate={handleChange}
        onEdit={handleChange}
        onMapRemove={handleChange}
        onMapCut={handleChange}
        onDragEnd={() => {
          setStatic('forceRedraw', !forceRedraw)
        }}
        onMarkerDragEnd={handleChange}
        onButtonClick={({ btnName }) => {
          setStatic(
            'cutMode',
            btnName === 'cutPolygon' && !map.pm.globalCutModeEnabled(),
          )
          setStatic(
            'dragMode',
            btnName === 'dragMode' && !map.pm.globalDragModeEnabled(),
          )
          setStatic(
            'editMode',
            btnName === 'editMode' && !map.pm.globalEditModeEnabled(),
          )
          setStatic(
            'removalMode',
            btnName === 'removalMode' && !map.pm.globalRemovalModeEnabled(),
          )
          setStatic(
            'rotateMode',
            btnName === 'rotateMode' && !map.pm.globalRotateModeEnabled(),
          )
        }}
      />
    </FeatureGroup>
  )
}
