/* eslint-disable react/destructuring-assignment */
import LineStringPopup from '@components/popups/LineString'
import StyledPopup from '@components/popups/Styled'
import { getColor } from '@services/utils'
import distance from '@turf/distance'
import type { Feature, LineString } from 'geojson'
import * as React from 'react'
import { Polyline } from 'react-leaflet'

export default function KojiLineString({
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
    <Polyline
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
      <StyledPopup>
        <LineStringPopup id={id} properties={properties} dis={dis} />
      </StyledPopup>
    </Polyline>
  )
}
