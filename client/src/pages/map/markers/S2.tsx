/* eslint-disable @typescript-eslint/ban-ts-comment */
import * as React from 'react'
import { Polygon, Tooltip, useMap } from 'react-leaflet'
import { S2Cell, S2CellId, S2Point, S2LatLng } from 'nodes2ts'
import { Feature, S2Response } from '@assets/types'
import { getS2Cells } from '@services/fetches'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'
import { VECTOR_COLORS } from '@assets/constants'
import union from '@turf/union'
import type { Polygon as PolygonType } from 'geojson'

function BaseCell({
  id,
  coords,
  covered,
  simple,
}: S2Response & { covered: boolean; simple?: boolean }) {
  return (
    <Polygon
      key={`${id}${covered}`}
      pmIgnore
      snapIgnore
      positions={coords}
      color={covered ? VECTOR_COLORS.RED : 'black'}
      fillOpacity={covered ? 0.2 : 0}
      weight={simple ? 2 : 0.5}
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
  const s2DisplayMode = usePersist((s) => s.s2DisplayMode)
  const s2FillMode = usePersist((s) => s.s2FillMode)
  const s2Level = usePersist((s) => s.s2_level)

  React.useEffect(() => {
    const signal = new AbortController()
    getS2Cells(map, level, signal.signal).then((res) => {
      if (res) {
        setData(res)
      }
    })
    return () => signal.abort()
  }, [location, s2DisplayMode, s2FillMode, s2Level])

  return s2FillMode === 'all' || s2DisplayMode === 'all' ? (
    <>
      {(s2DisplayMode === 'covered'
        ? data.filter((d) => covered[d.id]?.length)
        : data
      ).map((cell) => (
        <MemoBaseCell
          key={cell.id}
          {...cell}
          covered={s2FillMode === 'all' && !!covered[cell.id]?.length}
        />
      ))}
    </>
  ) : null
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

function SimplifiedCell({ cells }: { cells: string[] }) {
  const features: Feature<PolygonType>[] = cells
    .map((cell) => new S2CellId(cell))
    .map((id) => {
      const poly = []
      const cell = new S2Cell(id)
      for (let i = 0; i <= 3; i += 1) {
        const coordinate = cell.getVertex(i)
        const point = new S2Point(coordinate.x, coordinate.y, coordinate.z)
        const latLng = S2LatLng.fromPoint(point)
        poly.push([latLng.latDegrees, latLng.lngDegrees])
      }
      return {
        type: 'Feature',
        geometry: {
          type: 'Polygon',
          coordinates: [poly],
        },
        properties: {},
        id: id.id.toString(),
      }
    })

  // @ts-ignore
  const feature = features.reduce((acc, f) => {
    if (!acc) {
      return f
    }
    const combined = union(acc, f)
    if (combined) return combined
    return acc
  }, null as Feature<PolygonType> | null)

  return (
    <BaseCell
      id={feature.id}
      coords={feature.geometry.coordinates[0] as [number, number][]}
      covered
      simple
    />
  )
}

export const MemoSimplifiedCell = React.memo(SimplifiedCell, (prev, next) =>
  prev.cells.every((v, i) => v === next.cells[i]),
)

export function SimplifiedPolygons() {
  const covered = useShapes((s) => s.simplifiedS2Cells)
  const s2FillMode = usePersist((s) => s.s2FillMode)

  return s2FillMode === 'simple' ? (
    <>
      {Object.entries(covered).map(([id, cells]) => (
        <SimplifiedCell key={id} cells={cells} />
      ))}
    </>
  ) : null
}
