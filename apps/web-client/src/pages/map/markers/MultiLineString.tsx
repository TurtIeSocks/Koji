/* eslint-disable react/destructuring-assignment */
import * as React from 'react'
import type { MultiLineString } from 'geojson'
import type { Feature } from '@assets/types'

import { MemoLineString } from './LineString'

export function KojiMultiLineString({
  feature,
}: {
  feature: Feature<MultiLineString>
}) {
  return (
    <>
      {feature.geometry.coordinates.map((coords, i) => {
        return (
          <MemoLineString
            key={`multiLine_${coords}`}
            feature={{
              ...feature,
              id: `${i}__${i === coords.length - 1 ? 0 : i + 1}`,
              geometry: { type: 'LineString', coordinates: coords },
            }}
          />
        )
      })}
    </>
  )
}

export const MemoMultiLineString = React.memo(KojiMultiLineString)
