/* eslint-disable react/destructuring-assignment */
import type { Feature, Polygon as PolygonType } from 'geojson'
import * as React from 'react'
import { Polygon as BasePoly } from 'react-leaflet'

export default function Polygon({
  feature: {
    // id,
    properties,
    geometry: { coordinates },
  },
}: {
  feature: Feature<PolygonType>
}) {
  return (
    <BasePoly
      // key={id}
      positions={
        coordinates.map((each) => each.map(([lng, lat]) => [lat, lng])) as [
          [[number, number]],
        ]
      }
      {...properties}
      pmIgnore={false}
      pane="polygons"
    />
  )
}
