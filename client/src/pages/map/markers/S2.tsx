import * as React from 'react'
import { Polygon, Tooltip, useMap } from 'react-leaflet'
import { KojiResponse } from '@assets/types'
import { fetchWrapper } from '@services/fetches'
import { getMapBounds } from '@services/utils'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { VECTOR_COLORS } from '@assets/constants'

interface S2Response {
  id: string
  coords: [number, number][]
}

function BaseCell({ id, coords, covered }: S2Response & { covered: boolean }) {
  return (
    <Polygon
      key={`${id}${covered}`}
      pmIgnore
      snapIgnore
      positions={coords}
      color={covered ? VECTOR_COLORS.RED : 'black'}
      fillOpacity={covered ? 0.2 : 0}
      weight={0.5}
      pane="s2"
    >
      {process.env.NODE_ENV === 'development' && (
        <Tooltip direction="center">{id}</Tooltip>
      )}
    </Polygon>
  )
}

const MemoBaseCell = React.memo(
  BaseCell,
  (prev, next) => prev.id === next.id && prev.covered === next.covered,
)

function S2Level({
  level,
  location,
}: {
  level: number
  location: [number, number]
}) {
  const map = useMap()
  const [data, setData] = React.useState<S2Response[]>([])
  const covered = useShapes((s) => s.s2cellCoverage)

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
        if (res.data.length === 20_000) {
          useStatic.setState({
            networkStatus: {
              message: `Loaded the maximum of ${Number(
                20_000,
              ).toLocaleString()} Level ${level} S2 cells`,
              severity: 'warning',
              status: 200,
            },
          })
        }
        setData(res.data)
      }
    })
    return () => signal.abort()
  }, [location])

  return (
    <>
      {data.map((cell) => (
        <MemoBaseCell key={cell.id} {...cell} covered={!!covered[cell.id]} />
      ))}
    </>
  )
}

const MemoS2Cell = React.memo(
  S2Level,
  (prev, next) =>
    prev.level === next.level &&
    prev.location.every((v, i) => v === next.location[i]),
)

export function S2Cells() {
  const s2cells = usePersist((s) => s.s2cells)
  const bootstrap_mode = usePersist((s) => s.calculation_mode)
  const bootstrap_level = usePersist((s) => s.s2_level)
  const location = usePersist((s) => s.location)

  return (
    <>
      {(bootstrap_mode === 'Radius' ? s2cells : [bootstrap_level]).map(
        (level) => (
          <MemoS2Cell key={level} level={level} location={location} />
        ),
      )}
    </>
  )
}
