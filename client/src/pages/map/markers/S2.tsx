import * as React from 'react'
import { Polyline, useMap } from 'react-leaflet'
import { KojiResponse } from '@assets/types'
import { fetchWrapper } from '@services/fetches'
import { getMapBounds } from '@services/utils'
import { usePersist } from '@hooks/usePersist'

interface S2Response {
  id: string
  coords: [number, number][]
}

function BaseCell({ id, coords }: S2Response) {
  return (
    <Polyline
      key={id}
      positions={[...coords, coords[0]]}
      color="black"
      weight={0.5}
    />
  )
}

const MemoBaseCell = React.memo(BaseCell, (prev, next) => prev.id === next.id)

function S2Cell({
  level,
  location,
}: {
  level: number
  location: [number, number]
}) {
  const map = useMap()
  const [data, setData] = React.useState<S2Response[]>([])

  React.useEffect(() => {
    const signal = new AbortController()
    fetchWrapper<KojiResponse<S2Response[]>>(`/api/v1/s2/${level}`, {
      method: 'POST',
      body: JSON.stringify(getMapBounds(map)),
      headers: {
        'Content-Type': 'application/json',
      },
      signal: signal.signal,
    }).then((res) => {
      if (res) {
        setData(res.data)
      }
    })
    return () => signal.abort()
  }, [location])

  return (
    <>
      {data.map((cell) => (
        <MemoBaseCell key={cell.id} {...cell} />
      ))}
    </>
  )
}

const MemoS2Cell = React.memo(
  S2Cell,
  (prev, next) =>
    prev.level === next.level &&
    prev.location.every((v, i) => v === next.location[i]),
)

export function S2Cells() {
  const s2cells = usePersist((s) => s.s2cells)
  const location = usePersist((s) => s.location)

  return (
    <>
      {s2cells.map((level) => (
        <MemoS2Cell key={level} level={level} location={location} />
      ))}
    </>
  )
}
