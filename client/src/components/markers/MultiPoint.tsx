import * as React from 'react'
import type { Feature, MultiPoint as MpType } from 'geojson'
import Circle from './Circle'
import Polyline from './Polyline'

export default function MultiPoint({
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
            <Circle
              radius={radius}
              feature={{
                type: 'Feature',
                id: `${id}___${i}`,
                properties,
                geometry: { coordinates: first, type: 'Point' },
              }}
              type="multiPoints"
            />
            <Polyline
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
