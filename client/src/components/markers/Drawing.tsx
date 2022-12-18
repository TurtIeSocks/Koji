import * as React from 'react'
import { renderToString } from 'react-dom/server'
import { FeatureGroup, useMapEvents } from 'react-leaflet'
import useDeepCompareEffect from 'use-deep-compare-effect'
import * as L from 'leaflet'
import type { Feature, FeatureCollection, MultiPoint } from 'geojson'
import geohash from 'ngeohash'
import distance from '@turf/distance'

import { GeomanControls } from 'react-leaflet-geoman-v2'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'
import useSkipFirstEffect from '@hooks/useSkipFirstEffect'
import { getColor } from '@services/utils'

export function Drawing() {
  const snappable = useStore((s) => s.snappable)
  const continueDrawing = useStore((s) => s.continueDrawing)
  const radius = useStore((s) => s.radius)
  const showPolygons = useStore((s) => s.showPolygons)
  const showCircles = useStore((s) => s.showCircles)
  const setStore = useStore((s) => s.setStore)

  const layerEditing = useStatic((s) => s.layerEditing)
  const geojson = useStatic((s) => s.geojson)
  const setStatic = useStatic((s) => s.setStatic)
  const setGeojson = useStatic((s) => s.setGeojson)

  useMapEvents({
    click: (e) => setStatic('popupLocation', e.latlng),
    popupclose: () => setStatic('activeLayer', null),
  })

  const ref = React.useRef<L.FeatureGroup>(null)

  const handleChange = () => {
    const newGeo: FeatureCollection = {
      type: 'FeatureCollection',
      features: [],
    }
    const newMultiPointFeature: Feature<MultiPoint> = {
      geometry: { coordinates: [], type: 'MultiPoint' },
      properties: { name: 'new_circles', radius },
      type: 'Feature',
    }
    ref.current?.eachLayer((layer) => {
      if (
        layer instanceof L.Circle &&
        layer.feature?.properties.type === undefined
      ) {
        const { lat, lng } = layer.getLatLng()
        newMultiPointFeature.geometry.coordinates.push([lng, lat])
      } else if (layer instanceof L.Polygon) {
        newGeo.features.push(layer.toGeoJSON())
      }
    })
    if (newMultiPointFeature.geometry.coordinates.length) {
      newGeo.features.push(newMultiPointFeature)
    }
    setGeojson(newGeo)
  }

  const moveEvent: L.PM.DragEndEventHandler = ({ layer }) => {
    console.log(layer)
    if (layer instanceof L.Circle) {
      const feature: Feature<MultiPoint> | undefined = useStatic
        .getState()
        .geojson.features.find(
          (feat) =>
            feat.geometry.type === 'MultiPoint' &&
            layer.options.attribution === feat.properties?.name,
        ) as Feature<MultiPoint>
      if (feature && ref.current) {
        ref.current.eachLayer((lay) => {
          if (
            lay instanceof L.Polyline &&
            !(lay instanceof L.Polygon) &&
            lay.options.attribution === layer.options.attribution
          ) {
            ref.current?.removeLayer(lay)
          }
        })
        const { coordinates } = feature.geometry
        for (let i = 0; i < coordinates.length; i++) {
          const next =
            i === coordinates.length - 1 ? coordinates[0] : coordinates[i + 1]
          const dis = distance(coordinates[i], next, { units: 'meters' })

          L.polyline(
            [
              [coordinates[i][1], coordinates[i][0]],
              [next[1], next[0]],
            ],
            {
              color: getColor(dis),
              opacity: 80,
              pmIgnore: true,
              snapIgnore: true,
              pane: 'lines',
              attribution: layer.options.attribution,
            },
          )
            .addTo(ref.current)
            .bindPopup(renderToString(<div>{dis.toFixed(2)}m</div>))
        }
      }
    }

    handleChange()
  }

  useDeepCompareEffect(() => {
    if (Object.values(layerEditing).every((v) => !v)) {
      if (ref.current) {
        ref.current.eachLayer((layer) => {
          if (
            (layer instanceof L.Circle || layer instanceof L.Polygon) &&
            layer.feature?.properties?.type
          ) {
            ref.current?.removeLayer(layer)
          }
        })
      }
      L.geoJSON(geojson).eachLayer((layer) => {
        if (ref.current) {
          if (
            layer instanceof L.LayerGroup &&
            layer.feature?.type === 'Feature' &&
            layer.feature?.properties?.type
          ) {
            if (layer?.feature?.geometry?.type === 'MultiPoint') {
              const {
                geometry: { coordinates },
              } = layer.feature
              for (let i = 0; i < coordinates.length; i++) {
                const next =
                  i === coordinates.length - 1
                    ? coordinates[0]
                    : coordinates[i + 1]
                const dis = distance(coordinates[i], next, { units: 'meters' })

                const newCircle = L.circle(
                  [coordinates[i][1], coordinates[i][0]],
                  {
                    radius: radius || undefined,
                    snapIgnore: true,
                    pane: 'circles',
                    attribution: layer.feature?.properties.name,
                  },
                ).bindPopup(
                  renderToString(
                    <div>
                      Lat: {coordinates[i][0].toFixed(6)}
                      <br />
                      Lng: {coordinates[i][1].toFixed(6)}
                      <br />
                      Hash:{' '}
                      {geohash.encode(coordinates[i][1], coordinates[i][0], 9)}
                      <br />
                      Hash:{' '}
                      {geohash.encode(coordinates[i][1], coordinates[i][0], 12)}
                    </div>,
                  ),
                )
                newCircle.feature = {
                  ...layer.feature,
                  geometry: { type: 'Point', coordinates: coordinates[i] },
                }
                // L.PM.reInitLayer(newCircle)
                // layerEvents(layer, { onDragEnd: moveEvent }, 'on')
                newCircle.addTo(ref.current)

                // console.log(newCircle.getEvents?.())
                const line = L.polyline(
                  [
                    [coordinates[i][1], coordinates[i][0]],
                    [next[1], next[0]],
                  ],
                  {
                    color: getColor(dis),
                    opacity: 80,
                    pmIgnore: true,
                    snapIgnore: true,
                    pane: 'lines',
                    attribution: layer.feature?.properties.name,
                  },
                )
                  .addTo(ref.current)
                  .bindPopup(renderToString(<div>{dis.toFixed(2)}m</div>))
                line.feature = line.toGeoJSON()
                line.feature.properties.name = layer.feature.properties.name
              }
            }
          } else if (
            layer instanceof L.Polygon &&
            layer.feature?.properties?.type
          ) {
            layer.setStyle({ pane: 'polygons' })
            layer.on('click', () => setStatic('activeLayer', layer))
            ref.current.addLayer(layer)
          }
        }
      })
    }
    // eslint-disable-next-line no-console
    console.log({ geojson })
  }, [geojson])

  useSkipFirstEffect(() => {
    if (ref.current && radius) {
      ref.current?.getLayers().forEach((layer) => {
        if (layer instanceof L.Circle) {
          layer.setRadius(radius)
        }
      })
    }
  }, [radius])

  return (
    <FeatureGroup ref={ref}>
      <GeomanControls
        // eventDebugFn={
        //   // eslint-disable-next-line no-console
        //   process.env.NODE_ENV === 'development' ? console.log : undefined
        // }
        options={{
          position: 'topright',
          drawText: false,
          drawMarker: false,
          drawCircleMarker: false,
          drawCircle: true,
          drawRectangle: false,
          drawPolyline: false,
          drawPolygon: true,
        }}
        globalOptions={{
          continueDrawing,
          snappable,
          radiusEditCircle: false,
          templineStyle: { radius: radius || 70 },
          // panes: {
          //   layerPane: 'polygons',
          // },
        }}
        onCreate={({ layer }) => {
          if (layer instanceof L.Polygon) {
            layer.on('click', () => setStatic('activeLayer', layer))
            layer.setStyle({ pane: 'polygons' })
            ref.current?.removeLayer(layer)
            ref.current?.addLayer(layer)
            handleChange()
          }
          if (layer instanceof L.Circle) {
            if (ref.current) {
              layer.setStyle({
                snapIgnore: true,
                pane: 'circles',
                attribution: 'new_circles',
              })
              ref.current?.removeLayer(layer)
              ref.current?.addLayer(layer)

              if (radius) {
                layer.setRadius(radius)
              }
              const { lat, lng } = layer.getLatLng()
              layer.bindPopup(
                renderToString(
                  <div>
                    Lat: {lat.toFixed(6)}
                    <br />
                    Lng: {lng.toFixed(6)}
                    <br />
                    Hash: {geohash.encode(lng, lat, 9)}
                    <br />
                    Hash: {geohash.encode(lng, lat, 12)}
                  </div>,
                ),
              )
              const layers = ref.current
                ?.getLayers()
                .filter(
                  (l) => l instanceof L.Circle && !l.feature?.properties?.type,
                )

              if (layers && layers.length > 1) {
                const prev = layers.at(-2)
                const [first] = layers
                if (prev && prev instanceof L.Circle) {
                  const { lat: lat2, lng: lng2 } = prev.getLatLng()
                  const dis = distance([lng, lat], [lng2, lat2], {
                    units: 'meters',
                  })
                  L.polyline(
                    [
                      [lat2, lng2],
                      [lat, lng],
                    ],
                    {
                      color: getColor(dis),
                      opacity: 80,
                      pmIgnore: true,
                      snapIgnore: true,
                      pane: 'lines',
                      attribution: 'new_circles',
                    },
                  )
                    .addTo(ref.current)
                    .bindPopup(renderToString(<div>{dis.toFixed(2)}m</div>))
                  if (first && first instanceof L.Circle) {
                    const oldLayer = ref.current
                      .getLayers()
                      .find((x) => x.getAttribution?.() === 'last')
                    if (oldLayer) {
                      oldLayer.removeFrom(ref.current as unknown as L.Map)
                    }
                    const { lat: lat1, lng: lng1 } = first.getLatLng()
                    const dis2 = distance([lng, lat], [lng1, lat1], {
                      units: 'meters',
                    })
                    L.polyline(
                      [
                        [lat1, lng1],
                        [lat, lng],
                      ],
                      {
                        color: getColor(dis2),
                        opacity: 80,
                        pmIgnore: true,
                        snapIgnore: true,
                        pane: 'lines',
                        attribution: 'last',
                      },
                    )
                      .addTo(ref.current)
                      .bindPopup(renderToString(<div>{dis2.toFixed(2)}m</div>))
                  }
                }
              }
            }
          }
        }}
        onEdit={handleChange}
        onMapRemove={handleChange}
        onMapCut={handleChange}
        onDragEnd={moveEvent}
        onGlobalDrawModeToggled={({ enabled, shape }) => {
          if (!enabled) {
            setStatic('activeLayer', null)
          }
          if (shape === 'Polygon' && !showPolygons) {
            setStore('showPolygons', true)
          } else if (shape === 'Circle' && !showCircles) {
            setStore('showCircles', true)
          }
          if (shape === 'Circle' && !enabled) {
            handleChange()
          }
          setStatic('layerEditing', (e) => ({ ...e, drawMode: enabled }))
        }}
        onGlobalCutModeToggled={({ enabled }) =>
          setStatic('layerEditing', (e) => ({ ...e, cutMode: enabled }))
        }
        onGlobalDragModeToggled={({ enabled }) =>
          setStatic('layerEditing', (e) => ({ ...e, dragMode: enabled }))
        }
        onGlobalEditModeToggled={({ enabled }) =>
          setStatic('layerEditing', (e) => ({ ...e, editMode: enabled }))
        }
        onGlobalRemovalModeToggled={({ enabled }) =>
          setStatic('layerEditing', (e) => ({ ...e, removalMode: enabled }))
        }
        onGlobalRotateModeToggled={({ enabled }) =>
          setStatic('layerEditing', (e) => ({ ...e, rotateMode: enabled }))
        }
      />
    </FeatureGroup>
  )
}

export default React.memo(Drawing, () => true)
