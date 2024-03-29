import * as React from 'react'
import { LayerGroup, LayersControl, Rectangle } from 'react-leaflet'
import * as L from 'leaflet'
import center from '@turf/center'

import Map from '@components/Map'
import { useStatic } from '@hooks/useStatic'
import type { AllGeoJSON } from '@turf/helpers'
import type { FeatureCollection } from 'assets/types'
import GeoJsonWrapper from '@components/GeojsonWrapper'

export default function MiniMap({ filtered }: { filtered: FeatureCollection }) {
  const centerPoint = filtered.features.length
    ? center(filtered as AllGeoJSON)
    : { geometry: { coordinates: [0, 0] } }

  return (
    <Map
      forcedLocation={[
        centerPoint.geometry.coordinates[1],
        centerPoint.geometry.coordinates[0],
      ]}
      style={{ width: '100%', height: '50vh' }}
    >
      <LayersControl position="topright">
        <LayersControl.Overlay name="Collection BBox">
          {filtered.bbox && (
            <Rectangle
              bounds={[
                [filtered.bbox[1], filtered.bbox[0]],
                [filtered.bbox[3], filtered.bbox[2]],
              ]}
            />
          )}
        </LayersControl.Overlay>
        <LayersControl.Overlay name="Feature BBox">
          <LayerGroup>
            {filtered.features.map((feat) =>
              feat.bbox ? (
                <Rectangle
                  key={`${feat.bbox}`}
                  bounds={[
                    [feat.bbox[1], feat.bbox[0]],
                    [feat.bbox[3], feat.bbox[2]],
                  ]}
                />
              ) : null,
            )}
          </LayerGroup>
        </LayersControl.Overlay>
        <LayersControl.Overlay name="Features" checked>
          <GeoJsonWrapper
            data={filtered}
            onEachFeature={(feature, layer) => {
              if (layer instanceof L.Polygon && feature.properties) {
                layer.setStyle(feature.properties)
              }
              layer.bindTooltip(
                `
        <div><strong>Name:</strong> ${feature?.properties?.name}</div>
        <div><strong>Mode:</strong> ${feature?.properties?.mode}</div>
        <div>
        <strong>Projects:</strong>
          <p style="margin:0;padding:0">
            ${(feature?.properties?.projects || [])
              ?.map((p: number) => useStatic.getState().projects[p].name)
              .join(', ')}
          </p>
        </div>
        `,
                { direction: 'top' },
              )
            }}
          />
        </LayersControl.Overlay>
      </LayersControl>
    </Map>
  )
}
