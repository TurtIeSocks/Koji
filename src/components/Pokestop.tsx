import React from 'react'
import { CircleMarker, useMap, Popup } from 'react-leaflet'
import { useQuery } from '@apollo/client'
import { pokestop as Pokestop } from '@prisma/client'

import { pokestops } from '@services/queries'
import { getMapBounds } from '@services/utils'

function PopupContent({ poi }: { poi: Pokestop }) {
  const { name, lat, lon, updated } = poi

  return (
    <div>
      <h6>{name}</h6>
      <p>{updated}</p>
      <p>{`${lat.toFixed(6)}, ${lon.toFixed(6)}`}</p>
    </div>
  )
}

const Memoized = React.memo(
  ({ poi }: { poi: Pokestop }) => (
    <CircleMarker
      center={[poi.lat, poi.lon]}
      radius={5}
      pathOptions={{
        fillColor: 'green',
        fillOpacity: 1,
        opacity: 1,
        color: 'black',
        weight: 1,
      }}
    >
      <Popup>
        <PopupContent poi={poi} />
      </Popup>
    </CircleMarker>
  ),
  () => true,
)

export default function Pokestops() {
  const map = useMap()
  const { data, previousData } = useQuery<{ pokestops: Pokestop[] }>(
    pokestops,
    {
      variables: getMapBounds(map),
    },
  )

  const renderedData = data || previousData || { pokestops: [] }
  return (
    <>
      {renderedData.pokestops.map((poi) => (
        <Memoized key={poi.id} poi={poi} />
      ))}
    </>
  )
}
