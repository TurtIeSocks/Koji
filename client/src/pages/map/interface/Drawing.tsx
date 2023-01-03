/* eslint-disable no-param-reassign */
import * as React from 'react'
import { FeatureGroup, useMap } from 'react-leaflet'
import * as L from 'leaflet'
import type { Feature, MultiPolygon, Point, Polygon } from 'geojson'
import 'leaflet-arrowheads'

import { GeomanControls } from 'react-leaflet-geoman-v2'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'

export function Drawing() {
  const snappable = usePersist((s) => s.snappable)
  const continueDrawing = usePersist((s) => s.continueDrawing)
  const radius = usePersist((s) => s.radius)

  const map = useMap()

  const ref = React.useRef<L.FeatureGroup>(null)

  React.useEffect(() => {
    map.pm.Toolbar.changeActionsOfControl('removalMode', [
      {
        text: 'Lines',
        onClick() {
          useShapes.getState().setters.remove('LineString')
          useShapes.getState().setters.remove('MultiLineString')
        },
      },
      {
        text: 'Circles',
        onClick() {
          useShapes.getState().setters.remove('Point')
          useShapes.getState().setters.remove('MultiPoint')
        },
      },
      {
        text: 'Polygons',
        onClick() {
          useShapes.getState().setters.remove('Polygon')
          useShapes.getState().setters.remove('MultiPolygon')
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
                    setShapes('Polygon', (prev) => ({
                      ...prev,
                      [id]: feature as Feature<Polygon>,
                    }))
                  } else if (feature.geometry.type === 'MultiPolygon') {
                    setShapes('MultiPolygon', (prev) => ({
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
                    setShapes('LineString', (prev) => {
                      const newState: typeof prev = {
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
                      }
                      if (Object.keys(useShapes.getState().Point).length > 1) {
                        newState[`${feature.id}_${first.id}`] = {
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
                        }
                      }
                      return newState
                    })
                    setters.remove('LineString', `${last.id}_${first.id}`)
                  }
                  setShapes('Point', (prev) => ({
                    ...prev,
                    [id]: feature,
                  }))
                  if (last) {
                    setShapes('Point', (prev) => ({
                      ...prev,
                      [last?.id as number]: last,
                    }))
                  }
                  if (!first) {
                    setShapes('firstPoint', id)
                  } else {
                    setShapes('Point', (prev) => ({
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
          const { showCircles, showPolygons, setStore } = usePersist.getState()
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
