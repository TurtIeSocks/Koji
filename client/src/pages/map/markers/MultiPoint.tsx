import * as React from 'react'
import type { Feature, MultiPoint } from 'geojson'

import { MemoPoint } from './Point'
import { MemoLineString } from './LineString'

export function KojiMultiPoint({
  feature: {
    id,
    properties,
    geometry: { coordinates },
  },
  radius,
}: {
  feature: Feature<MultiPoint>
  radius: number
}) {
  return (
    <>
      {coordinates.map((first, i) => {
        if (first.length !== 2 && first.every((x) => typeof x === 'number')) {
          return null
        }
        const isEnd = i === coordinates.length - 1
        const next = isEnd ? coordinates[0] : coordinates[i + 1]
        return (
          <React.Fragment key={`${id}_${first}-${isEnd}`}>
            <MemoPoint
              radius={radius}
              feature={{
                type: 'Feature',
                id: `${id}___${i}`,
                properties,
                geometry: { coordinates: first, type: 'Point' },
              }}
              type="MultiPoint"
            />
            <MemoLineString
              key={`${first}-${isEnd}-${coordinates.length}`}
              feature={{
                type: 'Feature',
                properties,
                id: `${first}-${isEnd}`,
                geometry: { coordinates: [first, next], type: 'LineString' },
              }}
            />
          </React.Fragment>
        )
      })}
    </>
  )
}

export const MemoMultiPoint = React.memo(KojiMultiPoint)
