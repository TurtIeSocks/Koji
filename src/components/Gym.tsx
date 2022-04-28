import React from 'react'
import { CircleMarker, useMap, Popup } from 'react-leaflet'
import { useQuery } from '@apollo/client'
import { gym as Gym } from '@prisma/client'

import { gyms } from '@services/queries'
import { getMapBounds } from '@services/utils'

function PopupContent({ poi }: { poi: Gym }) {
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
  ({ poi }: { poi: Gym }) => (
    <CircleMarker
      center={[poi.lat, poi.lon]}
      radius={7}
      pathOptions={{
        fillColor: 'red',
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

export default function Gyms() {
  const map = useMap()
  const { data, previousData } = useQuery<{ gyms: Gym[] }>(gyms, {
    variables: getMapBounds(map),
  })

  const renderedData = data || previousData || { gyms: [] }
  return (
    <>
      {renderedData.gyms.map((poi) => (
        <Memoized key={poi.id} poi={poi} />
      ))}
    </>
  )
}
