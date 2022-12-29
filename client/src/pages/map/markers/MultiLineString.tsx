/* eslint-disable react/destructuring-assignment */
import type { Feature, MultiLineString } from 'geojson'
import * as React from 'react'
import KojiLineString from './LineString'

export default function KojiMultiLineString({
  feature,
}: {
  feature: Feature<MultiLineString>
}) {
  return (
    <>
      {feature.geometry.coordinates.map((coords) => (
        <KojiLineString
          key={`multiLine_${coords}`}
          feature={{
            ...feature,
            geometry: { type: 'LineString', coordinates: coords },
          }}
        />
      ))}
    </>
  )
}
