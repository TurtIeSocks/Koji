/* eslint-disable no-param-reassign */
import * as React from 'react'
import { FeatureGroup, useMap } from 'react-leaflet'
import * as L from 'leaflet'
import type { MultiPolygon, Point, Polygon } from 'geojson'
import 'leaflet-arrowheads'
import { GeomanControls } from 'react-leaflet-geoman-v2'

import type { Feature } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'

export function Drawing() {
  const snappable = usePersist((s) => s.snappable)
  const continueDrawing = usePersist((s) => s.continueDrawing)
  const radius = usePersist((s) => s.radius)

  const map = useMap()

  const ref = React.useRef<L.FeatureGroup>(null)

  const revertPolygonsToDefault = () =>
    map.pm.getGeomanLayers().forEach((layer) => {
      if (layer instanceof L.Polygon) {
        layer.setStyle({ color: '#3388ff' })
      }
    })

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
          customControls: true,
        }}
        globalOptions={{
          continueDrawing,
          snappable,
          radiusEditCircle: false,
          templineStyle: { radius: radius || 70 },
        }}
        onMount={() => {
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
          map.pm.Toolbar.changeActionsOfControl('drawCircle', [
            {
              text: 'Finish',
              onClick() {
                map.pm.disableGlobalRemovalMode()
                useShapes
                  .getState()
                  .setters.activeRoute(
                    `new_route_${useShapes.getState().newRouteCount + 1}`,
                  )
                useShapes
                  .getState()
                  .setShapes('newRouteCount', (prev) => prev + 1)
              },
            },
            {
              text: 'New Route',
              onClick() {
                useShapes
                  .getState()
                  .setters.activeRoute(
                    `new_route_${useShapes.getState().newRouteCount + 1}`,
                  )
                useShapes
                  .getState()
                  .setShapes('newRouteCount', (prev) => prev + 1)
              },
            },
            {
              text: 'Cancel',
              onClick() {
                map.pm.disableGlobalRemovalMode()
                useShapes.getState().setters.remove('Point')
                useShapes.getState().setters.remove('LineString')
              },
            },
          ])
          if (!map.pm.Toolbar.controlExists('mergeMode')) {
            map.pm.Toolbar.createCustomControl({
              name: 'mergeMode',
              block: 'custom',
              title: 'Merge Polygons',
              className: 'leaflet-button-merge',
              toggle: true,
              actions: [
                {
                  text: 'Merge',
                  onClick() {
                    useShapes.getState().setters.combine()
                  },
                },
                'cancel',
              ],
            })
          }
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
                  feature.id = id.toString()
                  if (feature.geometry.type === 'Polygon') {
                    setShapes('Polygon', (prev) => ({
                      ...prev,
                      [id.toString()]: feature as Feature<Polygon>,
                    }))
                  } else if (feature.geometry.type === 'MultiPolygon') {
                    setShapes('MultiPolygon', (prev) => ({
                      ...prev,
                      [id.toString()]: feature as Feature<MultiPolygon>,
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
                    if (typeof first?.id === 'number') {
                      feature.properties.__forward = first.id
                    }
                    if (typeof last?.id === 'number') {
                      feature.properties.__backward = last.id
                    }
                  }
                  if (last?.properties) {
                    last.properties.__forward = id
                  }
                  if (first?.properties) {
                    first.properties.__backward = id
                  }
                  if (last && first) {
                    setShapes('LineString', (prev) => {
                      const newState: typeof prev = {
                        ...prev,
                        [`${+last.id}__${+feature.id}`]: {
                          type: 'Feature',
                          id: `${+last.id}__${+feature.id}`,
                          properties: {
                            __start: +last.id,
                            __end: +feature.id,
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
                        if (
                          typeof feature.id === 'number' &&
                          typeof first.id === 'number'
                        ) {
                          newState[`${feature.id}__${first.id}`] = {
                            type: 'Feature',
                            id: `${feature.id}__${first.id}`,
                            properties: {
                              __start: feature.id,
                              __end: first.id,
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
                      }
                      return newState
                    })
                    setters.remove('LineString', `${last.id}__${first.id}`)
                  }
                  setShapes('Point', (prev) => ({
                    ...prev,
                    [id]: feature,
                  }))
                  if (last) {
                    setShapes('Point', (prev) => ({
                      ...prev,
                      [last?.id]: last,
                    }))
                  }
                  if (!first) {
                    setShapes('firstPoint', id)
                  } else {
                    setShapes('Point', (prev) => ({
                      ...prev,
                      [first?.id]: first,
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
          const { setters, setShapes, newRouteCount } = useShapes.getState()

          switch (shape) {
            case 'Circle':
              if (!showCircles && enabled) {
                setStore('showCircles', true)
              }
              setters.activeRoute(`new_route_${newRouteCount + 1}`)
              setShapes('newRouteCount', (prev) => prev + 1)
              break
            case 'Rectangle':
            case 'Polygon':
              if (!showPolygons && enabled) {
                setStore('showPolygons', true)
              }
              break
            default:
              break
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
        onGlobalEditModeToggled={({ enabled }) => {
          return useStatic
            .getState()
            .setStatic('layerEditing', (e) => ({ ...e, editMode: enabled }))
        }}
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
        onButtonClick={(e) => {
          revertPolygonsToDefault()
          if (e.btnName === 'mergeMode') {
            useShapes.setState({ combined: {} })
            useStatic.setState((prev) => ({
              combinePolyMode: !prev.combinePolyMode,
            }))
          } else {
            useStatic.setState({ combinePolyMode: false })
          }
        }}
        onActionClick={({ btnName, text }) => {
          if (btnName === 'mergeMode' && text === 'Cancel') {
            revertPolygonsToDefault()
            useStatic.setState({ combinePolyMode: false })
            useShapes.setState({ combined: {} })
          }
        }}
      />
    </FeatureGroup>
  )
}

export default React.memo(Drawing, () => true)
