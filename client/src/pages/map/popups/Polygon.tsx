import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { Button, CircularProgress, Divider, Typography } from '@mui/material'
import useDeepCompareEffect from 'use-deep-compare-effect'
import type { Feature } from 'geojson'

import ExportPolygon from '@components/dialogs/Polygon'
import { getData } from '@services/fetches'
import { useShapes } from '@hooks/useShapes'

export function PolygonPopup({
  feature: ref,
  loadData,
}: {
  feature: Feature
  loadData: boolean
}) {
  const feature = useShapes((s) =>
    ref.geometry.type === 'Polygon'
      ? s.Polygon[ref.id as number | string]
      : s.MultiPolygon[ref.id as number | string],
  ) ||
    ref || { properties: {}, geometry: { type: 'Polygon', coordinates: [] } }
  const remove = useShapes((s) => s.setters.remove)

  const [open, setOpen] = React.useState('')
  const [active, setActive] = React.useState<{
    spawnpoint: number | null | string
    gym: number | null | string
    pokestop: number | null | string
  }>({
    spawnpoint: null,
    gym: null,
    pokestop: null,
  })

  const getState = (category: keyof typeof active) => {
    switch (typeof active[category]) {
      case 'number':
        return active[category]?.toLocaleString()
      case 'string':
        return active[category]
      case 'object':
        return <CircularProgress size={10} />
      default:
        return 'Loading'
    }
  }

  useDeepCompareEffect(() => {
    if (feature.geometry.coordinates.length && loadData) {
      Promise.allSettled(
        ['pokestop', 'gym', 'spawnpoint'].map((category) =>
          getData<{ total: number }>(`/internal/data/area_stats/${category}`, {
            area: feature,
          }).then((data) =>
            setActive((prev) => ({
              ...prev,
              [category]: data?.total ?? (data || 0),
            })),
          ),
        ),
      )
    }
  }, [feature, loadData])

  return feature ? (
    <>
      <Grid2 container minWidth={150}>
        <Grid2 xs={12}>
          <Typography>{feature.properties?.name}</Typography>
          <Typography>{feature.properties?.type}</Typography>
        </Grid2>
        <Divider
          flexItem
          sx={{ my: 1, color: 'black', width: '90%', height: 2 }}
        />
        <Grid2 xs={12}>
          <Typography>Pokestops: {getState('pokestop')}</Typography>
          <Typography>Gyms: {getState('gym')}</Typography>
          <Typography>Spawnpoints: {getState('spawnpoint')}</Typography>
        </Grid2>
        <Grid2>
          <Button size="small" onClick={() => setOpen('polygon')}>
            Export Polygon
          </Button>
          <br />
          <Button
            size="small"
            onClick={() => remove(feature.geometry.type, feature.id)}
          >
            Remove
          </Button>
        </Grid2>
      </Grid2>
      {open && (
        <ExportPolygon
          mode="export"
          open={open}
          setOpen={setOpen}
          feature={feature}
        />
      )}
    </>
  ) : null
}

export const MemoPolyPopup = React.memo(
  PolygonPopup,
  (prev, next) => prev.loadData === next.loadData,
)
