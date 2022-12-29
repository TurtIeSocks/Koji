/* eslint-disable react/destructuring-assignment */
import * as React from 'react'
import type { Feature, MultiLineString } from 'geojson'

import { MemoLineString } from './LineString'

export function KojiMultiLineString({
  feature,
}: {
  feature: Feature<MultiLineString>
}) {
  return (
    <>
      {feature.geometry.coordinates.map((coords) => (
        <MemoLineString
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

export const MemoMultiLineString = React.memo(KojiMultiLineString)
