import React from 'react'
import { useMap, Popup, CircleMarker } from 'react-leaflet'
import { useQuery } from '@apollo/client'
import { spawnpoint as Spawnpoint } from '@prisma/client'

import { spawnpoints } from '@services/queries'
import { getMapBounds } from '@services/utils'

function PopupContent({ point }: { point: Spawnpoint }) {
  const { despawn_sec, lat, lon, updated } = point
  const despawn = despawn_sec || 0
  const minute = despawn > 60 ? Math.round(despawn / 60) : despawn
  const minuteFixed = minute < 10 ? `0${minute}` : minute

  return (
    <div>
      <h6>{despawn ? `00:${minuteFixed}` : '?'}</h6>
      <p>{updated}</p>
      <p>{`${lat.toFixed(6)}, ${lon.toFixed(6)}`}</p>
    </div>
  )
}

const Memoized = React.memo(
  ({ point }: { point: Spawnpoint }) => (
    <CircleMarker
      center={[point.lat, point.lon]}
      radius={3}
      pathOptions={{
        fillColor: point.despawn_sec ? 'deeppink' : 'dodgerblue',
        fillOpacity: 1,
        opacity: 1,
        color: 'black',
        weight: 1,
      }}
    >
      <Popup>
        <PopupContent point={point} />
      </Popup>
    </CircleMarker>
  ),
  (prev: { point: Spawnpoint }, next: { point: Spawnpoint }) =>
    prev.point.despawn_sec === next.point.despawn_sec,
)

export default function Spawnpoints() {
  const map = useMap()
  const { data, previousData } = useQuery<{ spawnpoints: Spawnpoint[] }>(
    spawnpoints,
    {
      variables: getMapBounds(map),
    },
  )

  const renderedData = data || previousData || { spawnpoints: [] }
  return (
    <>
      {renderedData.spawnpoints.map((point) => (
        <Memoized key={point.id.toString()} point={point} />
      ))}
    </>
  )
}
