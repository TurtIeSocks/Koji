/* eslint-disable react/destructuring-assignment */
import * as React from 'react'
import { Polyline } from 'react-leaflet'
import type { Feature, LineString } from 'geojson'
import distance from '@turf/distance'

import { getColor } from '@services/utils'

import { MemoLinePopup } from '../popups/LineString'
import Popup from '../popups/Styled'

export function KojiLineString({
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
            pane: 'arrows',
            opacity,
            pmIgnore: true,
            snapIgnore: true,
            size: '50m',
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
        <MemoLinePopup id={id} properties={properties} dis={dis} />
      </Popup>
    </Polyline>
  )
}

export const MemoLineString = React.memo(
  KojiLineString,
  (prev, next) => prev.feature.id === next.feature.id,
)
