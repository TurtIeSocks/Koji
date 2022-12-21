/* eslint-disable no-param-reassign */
import * as React from 'react'
import { FeatureGroup, useMapEvents } from 'react-leaflet'
import * as L from 'leaflet'
import type { Feature, MultiPolygon, Point, Polygon } from 'geojson'
import 'leaflet-arrowheads'

import { GeomanControls } from 'react-leaflet-geoman-v2'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'
import { useShapes } from '@hooks/useShapes'

export function Drawing() {
  const snappable = useStore((s) => s.snappable)
  const continueDrawing = useStore((s) => s.continueDrawing)
  const radius = useStore((s) => s.radius)

  const map = useMapEvents({
    click: (e) => useStatic.getState().setStatic('popupLocation', e.latlng),
    popupclose: () => useStatic.getState().setStatic('activeLayer', null),
  })

  const ref = React.useRef<L.FeatureGroup>(null)

  React.useEffect(() => {
    map.pm.Toolbar.changeActionsOfControl('removalMode', [
      {
        text: 'Lines',
        onClick() {
          useShapes.getState().setters.remove('lineStrings')
          useShapes.getState().setters.remove('multiLineStrings')
        },
      },
      {
        text: 'Circles',
        onClick() {
          useShapes.getState().setters.remove('points')
          useShapes.getState().setters.remove('multiPoints')
        },
      },
      {
        text: 'Polygons',
        onClick() {
          useShapes.getState().setters.remove('polygons')
          useShapes.getState().setters.remove('multiPolygons')
        },
      },
      {
        text: 'Finish',
        onClick() {
          map.pm.disableGlobalRemovalMode()
        },
      },
    ])
  }, [])

  return (
    <FeatureGroup ref={ref}>
      <GeomanControls
        options={{
          position: 'topright',
          drawText: false,
          drawMarker: false,
          drawCircleMarker: false,
          drawCircle: true,
          drawRectangle: true,
          drawPolyline: false,
          drawPolygon: true,
        }}
        globalOptions={{
          continueDrawing,
          snappable,
          radiusEditCircle: false,
          templineStyle: { radius: radius || 70 },
          panes: {
            polygonPane: 'polygons',
            circlePane: 'circles',
            polylinePane: 'lines',
          },
        }}
        onCreate={({ layer, shape }) => {
          if (ref.current && ref.current.hasLayer(layer)) {
            const id = ref.current.getLayerId(layer)
            const { setters, getters, setShapes } = useShapes.getState()
            switch (shape) {
              case 'Rectangle':
              case 'Polygon':
                if (layer instanceof L.Polygon) {
                  const feature = layer.toGeoJSON()
                  feature.id = id
                  if (feature.geometry.type === 'Polygon') {
                    setShapes('polygons', (prev) => ({
                      ...prev,
                      [id]: feature as Feature<Polygon>,
                    }))
                  } else if (feature.geometry.type === 'MultiPolygon') {
                    setShapes('multiPolygons', (prev) => ({
                      ...prev,
                      [id]: feature as Feature<MultiPolygon>,
                    }))
                  }
                }
                break
              case 'Circle':
                if (layer instanceof L.Circle) {
                  const feature = layer.toGeoJSON() as Feature<Point>
                  feature.id = id

                  const first = getters.getFirst()
                  const last = getters.getLast()

                  if (feature.properties) {
                    feature.properties.forward = first?.id
                    feature.properties.backward = last?.id
                  }
                  if (last?.properties) {
                    last.properties.forward = id
                  }
                  if (first?.properties) {
                    first.properties.backward = id
                  }
                  if (last && first) {
                    setShapes('lineStrings', (prev) => ({
                      ...prev,
                      [`${last.id}_${feature.id}`]: {
                        type: 'Feature',
                        id: `${last.id}_${feature.id}`,
                        properties: {
                          start: last.id,
                          end: feature.id,
                        },
                        geometry: {
                          type: 'LineString',
                          coordinates: [
                            last.geometry.coordinates,
                            feature.geometry.coordinates,
                          ],
                        },
                      },
                      [`${feature.id}_${first.id}`]: {
                        type: 'Feature',
                        id: `${feature.id}_${first.id}`,
                        properties: {
                          start: feature.id,
                          end: first.id,
                        },
                        geometry: {
                          type: 'LineString',
                          coordinates: [
                            feature.geometry.coordinates,
                            first.geometry.coordinates,
                          ],
                        },
                      },
                    }))
                    setters.remove('lineStrings', `${last.id}_${first.id}`)
                  }
                  setShapes('points', (prev) => ({
                    ...prev,
                    [id]: feature,
                  }))
                  if (last) {
                    setShapes('points', (prev) => ({
                      ...prev,
                      [last?.id as number]: last,
                    }))
                  }
                  if (!first) {
                    setShapes('firstPoint', id)
                  } else {
                    setShapes('points', (prev) => ({
                      ...prev,
                      [first?.id as number]: first,
                    }))
                  }
                  setShapes('lastPoint', id)
                }
                break
              default:
                break
            }
            ref.current.removeLayer(layer)
          }
        }}
        onGlobalDrawModeToggled={({ enabled, shape }) => {
          const { showCircles, showPolygons, setStore } = useStore.getState()
          if (!enabled) {
            useStatic.getState().setStatic('activeLayer', null)
          }
          if (shape === 'Polygon' && !showPolygons) {
            setStore('showPolygons', true)
          } else if (shape === 'Circle' && !showCircles) {
            setStore('showCircles', true)
          }
          useStatic
            .getState()
            .setStatic('layerEditing', (e) => ({ ...e, drawMode: enabled }))
        }}
        onGlobalCutModeToggled={({ enabled }) =>
          useStatic
            .getState()
            .setStatic('layerEditing', (e) => ({ ...e, cutMode: enabled }))
        }
        onGlobalDragModeToggled={({ enabled }) =>
          useStatic
            .getState()
            .setStatic('layerEditing', (e) => ({ ...e, dragMode: enabled }))
        }
        onGlobalEditModeToggled={({ enabled }) =>
          useStatic
            .getState()
            .setStatic('layerEditing', (e) => ({ ...e, editMode: enabled }))
        }
        onGlobalRemovalModeToggled={({ enabled }) =>
          useStatic
            .getState()
            .setStatic('layerEditing', (e) => ({ ...e, removalMode: enabled }))
        }
        onGlobalRotateModeToggled={({ enabled }) =>
          useStatic
            .getState()
            .setStatic('layerEditing', (e) => ({ ...e, rotateMode: enabled }))
        }
      />
    </FeatureGroup>
  )
}

export default React.memo(Drawing, () => true)
