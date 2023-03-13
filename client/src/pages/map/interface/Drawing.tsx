/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable no-param-reassign */
import * as React from 'react'
import { useMap } from 'react-leaflet'
import * as L from 'leaflet'
import type { MultiPolygon, Point, Polygon } from 'geojson'
import 'leaflet-arrowheads'
import { GeomanControls } from 'react-leaflet-geoman-v2'

import type { Feature } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'
import { buildShortcutKey, getKey, reverseObject } from '@services/utils'

export function Drawing() {
  const snappable = usePersist((s) => s.snappable)
  const continueDrawing = usePersist((s) => s.continueDrawing)
  const radius = usePersist((s) => s.radius)

  const map = useMap()

  const revertPolygonsToDefault = () =>
    map.pm.getGeomanLayers().forEach((layer) => {
      if (layer instanceof L.Polygon) {
        layer.setStyle({ color: '#3388ff' })
      }
    })

  return (
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
        // if (ref.current && ref.current.hasLayer(layer)) {
        const { setters, getters, setShapes } = useShapes.getState()
        switch (shape) {
          case 'Rectangle':
          case 'Polygon':
            if (layer instanceof L.Polygon) {
              const feature = layer.toGeoJSON()
              const id = getKey()
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
              const id = Object.values(useShapes.getState().Point).length * 10

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
        layer.remove()
        // }
      }}
      onMapCut={({ layer, originalLayer }) => {
        if (
          layer instanceof L.Polygon &&
          originalLayer instanceof L.Polygon &&
          originalLayer.feature?.id
        ) {
          const original = originalLayer.toGeoJSON()
          const newLayer = layer.toGeoJSON()
          layer.feature = {
            ...original,
            geometry: newLayer.geometry,
          }
          useShapes
            .getState()
            .setters.update(
              originalLayer.feature.geometry.type,
              originalLayer.feature.id,
              {
                ...newLayer,
                id: originalLayer.feature.id,
                properties: {
                  ...originalLayer.feature.properties,
                  __leaflet_id: layer._leaflet_id,
                },
              } as any,
            ) // TODO: fix this
          if (newLayer.geometry.type !== original.geometry.type) {
            layer.remove()
          }
        }
        // originalLayer.remove()
      }}
      onMapRotateEnd={({ layer }) => {
        if (layer instanceof L.Polygon && layer.feature?.id) {
          const feature = layer.toGeoJSON()
          useShapes
            .getState()
            .setters.update(feature.geometry.type, layer.feature?.id, {
              ...feature,
              id: feature.id,
              properties: feature.properties,
            } as any) // TODO: fix this
        }
      }}
      onDragEnd={({ layer }) => {
        if (layer instanceof L.Polygon && layer.feature?.id) {
          const feature = layer.toGeoJSON()
          useShapes
            .getState()
            .setters.update(feature.geometry.type, layer.feature?.id, {
              ...feature,
              id: feature.id,
              properties: feature.properties,
            } as any) // TODO: fix this
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
      onKeyEvent={(e) => {
        if (e.eventType === 'keyup' || useStatic.getState().dialogs.keyboard)
          return
        const { kbShortcuts, tileServer } = usePersist.getState()
        const { tileServers } = useStatic.getState()

        const reverse = reverseObject(kbShortcuts)
        const shortcut = buildShortcutKey(e.event)
        if (reverse[shortcut]) {
          e.event.preventDefault()
          switch (reverse[shortcut]) {
            case 'drawCircle':
              if (!map.pm.globalDrawModeEnabled()) {
                map.pm.enableDraw('Circle')
              } else {
                map.pm.disableDraw('Circle')
              }
              break
            case 'drawRectangle':
              if (!map.pm.globalDrawModeEnabled()) {
                map.pm.enableDraw('Rectangle')
              } else {
                map.pm.disableDraw('Rectangle')
              }
              break
            case 'drawPolygon':
              if (!map.pm.globalDrawModeEnabled()) {
                map.pm.enableDraw('Polygon')
              } else {
                map.pm.disableDraw('Polygon')
              }
              break
            case 'drag':
              map.pm.toggleGlobalDragMode()
              break
            case 'edit':
              map.pm.toggleGlobalEditMode()
              break
            case 'remove':
              map.pm.toggleGlobalRemovalMode()
              break
            case 'rotate':
              map.pm.toggleGlobalRotateMode()
              break
            case 'cut':
              map.pm.toggleGlobalCutMode()
              break
            case 'setTileServer':
              {
                const index = tileServers.findIndex(
                  (ts) => ts.url === tileServer,
                )
                usePersist.setState({
                  tileServer: tileServers[index + 1]
                    ? tileServers[index + 1].url
                    : tileServers[0].url,
                })
              }
              break
            case 'theme':
              usePersist.setState({
                darkMode: !usePersist.getState().darkMode,
              })
              break
            case 'drawer':
              usePersist.setState({
                drawer: !usePersist.getState().drawer,
              })
              break
            case 'arrows':
              usePersist.setState({
                showArrows: !usePersist.getState().showArrows,
              })
              break
            case 'circles':
              usePersist.setState({
                showCircles: !usePersist.getState().showCircles,
              })
              break
            case 'lines':
              usePersist.setState({
                showLines: !usePersist.getState().showLines,
              })
              break
            case 'polygons':
              usePersist.setState({
                showPolygons: !usePersist.getState().showPolygons,
              })
              break
            case 'gyms':
              usePersist.setState({
                gym: !usePersist.getState().gym,
              })
              break
            case 'pokestops':
              usePersist.setState({
                pokestop: !usePersist.getState().pokestop,
              })
              break
            case 'spawnpoints':
              usePersist.setState({
                spawnpoint: !usePersist.getState().spawnpoint,
              })
              break
            default:
          }
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
  )
}

export default React.memo(Drawing)
