import * as React from 'react'
import type { Feature, MultiPoint as MpType } from 'geojson'
import KojiPoint from './Point'
import KojiLineString from './LineString'

export default function KojiMultiPoint({
  feature: {
    id,
    properties,
    geometry: { coordinates },
  },
  radius,
}: {
  feature: Feature<MpType>
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
          <React.Fragment key={`${first}-${isEnd}`}>
            <KojiPoint
              radius={radius}
              feature={{
                type: 'Feature',
                id: `${id}___${i}`,
                properties,
                geometry: { coordinates: first, type: 'Point' },
              }}
              type="MultiPoint"
            />
            <KojiLineString
              feature={{
                type: 'Feature',
                properties,
                geometry: { coordinates: [first, next], type: 'LineString' },
              }}
            />
          </React.Fragment>
        )
      })}
    </>
  )
}
