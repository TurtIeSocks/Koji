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
import type { Point, Polygon as PolygonType } from 'geojson'
import { useStatic } from '@hooks/useStatic'

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
      eventHandlers={{
        click: () => {
          if (process.env.NODE_ENV === 'development') {
            navigator.clipboard.writeText(id)
          }
        },
      }}
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

function SimplifiedCell({
  point,
  cells,
}: {
  point: Feature<Point>
  cells: string[]
}) {
  const center = point?.geometry
    ? S2CellId.fromPoint(
        S2LatLng.fromDegrees(
          point.geometry.coordinates[1],
          point.geometry.coordinates[0],
        ).toPoint(),
      ).id.toString()
    : cells.join('').substring(0, 50)
  const features: Feature<PolygonType>[] = cells
    .map((cell) => new S2CellId(cell))
    .map((cellId) => {
      const poly = []
      const cell = new S2Cell(cellId)
      for (let i = 0; i < 4; i += 1) {
        const coordinate = cell.getVertex(i)
        const s2Point = new S2Point(coordinate.x, coordinate.y, coordinate.z)
        const latLng = S2LatLng.fromPoint(s2Point)
        poly.push([latLng.latDegrees, latLng.lngDegrees])
      }
      if (poly[0][0] !== poly[3][0] || poly[0][1] !== poly[3][1]) {
        poly.push(poly[0])
      }
      return {
        type: 'Feature',
        geometry: {
          type: 'Polygon',
          coordinates: [poly],
        },
        properties: {},
        id: cellId.id.toString(),
      }
    })
  // turns multiple polygon features into one
  // @ts-ignore
  const feature = features.reduce((acc, f) => {
    if (!acc) {
      return f
    }
    try {
      const combined = union(acc, f)
      if (combined) return combined
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error(e, '\n', f)
      if (e instanceof Error && process.env.NODE_ENV === 'development')
        useStatic.setState({
          notification: {
            message: e.message,
            severity: 'warning',
            status: 1,
          },
        })
    }
    return acc
  }, null as Feature<PolygonType> | null)

  return (
    <BaseCell
      id={center}
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
  const s2cellCoverage = useShapes((s) => s.s2cellCoverage)
  const s2FillMode = usePersist((s) => s.s2FillMode)
  const point = useShapes((s) => s.Point)
  const covered: Record<string, string[]> = {}
  Object.entries(s2cellCoverage).forEach(([cellId, pointIds]) =>
    pointIds.forEach((pointId) => {
      if (!covered[pointId]) covered[pointId] = []
      covered[pointId].push(cellId)
    }),
  )
  return s2FillMode === 'simple' ? (
    <>
      {Object.entries(covered).map(([id, cells]) => (
        <MemoSimplifiedCell
          key={`${id}${cells.length}`}
          point={point[id.split('__')[1]]}
          cells={cells}
        />
      ))}
    </>
  ) : null
}
