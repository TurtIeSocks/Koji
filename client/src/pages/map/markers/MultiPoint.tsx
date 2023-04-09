import * as React from 'react'
import type { MultiPoint } from 'geojson'

import type { Feature, DbOption, KojiKey } from '@assets/types'

import { MemoPoint } from './Point'
import { MemoLineString } from './LineString'

export function KojiMultiPoint({
  feature: {
    id,
    properties,
    geometry: { coordinates },
  },
  radius,
  dbRef,
  combined,
}: {
  feature: Feature<MultiPoint>
  combined?: boolean
  radius: number
  dbRef: DbOption | null
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
                id: i,
                properties: {
                  ...properties,
                  __multipoint_id: id as KojiKey,
                },
                geometry: { coordinates: first, type: 'Point' },
              }}
              index={i}
              type="MultiPoint"
              dbRef={dbRef}
              combined={combined}
            />
            <MemoLineString
              key={`${first}-${isEnd}-${coordinates.length}`}
              feature={{
                type: 'Feature',
                properties,
                id: `${i}__${isEnd ? 0 : i + 1}`,
                geometry: { coordinates: [first, next], type: 'LineString' },
              }}
            />
          </React.Fragment>
        )
      })}
    </>
  )
}

export const MemoMultiPoint = React.memo(
  KojiMultiPoint,
  (prev, next) =>
    prev.combined === next.combined && prev.radius === next.radius,
)
