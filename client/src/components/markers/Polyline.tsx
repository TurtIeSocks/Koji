/* eslint-disable react/destructuring-assignment */
import { getColor } from '@services/utils'
import distance from '@turf/distance'
import type { Feature, LineString } from 'geojson'
import * as React from 'react'
import { Polyline as BaseLine, Popup } from 'react-leaflet'

export default function Polyline({
  feature: {
    id,
    properties,
    geometry: { coordinates },
  },
  opacity = properties?.opacity || 0.8,
  fillOpacity = properties?.fillOpacity || opacity,
}: {
  opacity?: number
  fillOpacity?: number
  feature: Feature<LineString>
}) {
  const dis = distance(coordinates[0], coordinates[1], {
    units: 'meters',
  })
  const color = getColor(dis)

  return (
    <BaseLine
      key={dis}
      ref={(line) => {
        if (line && id) {
          line.arrowheads({
            color,
            fill: true,
            fillOpacity,
            pane: 'lines',
            opacity,
            pmIgnore: true,
            snapIgnore: true,
            size: '8px',
            offsets: { end: `${dis / 2}m` },
          })
        }
      }}
      positions={[
        [coordinates[0][1], coordinates[0][0]],
        [coordinates[1][1], coordinates[1][0]],
      ]}
      color={color}
      opacity={opacity}
      fillOpacity={fillOpacity}
      pmIgnore
      snapIgnore
      pane="lines"
    >
      <Popup>
        {JSON.stringify({ id, properties }, null, 2)}
        {dis.toFixed(2)}
      </Popup>
    </BaseLine>
  )
}
