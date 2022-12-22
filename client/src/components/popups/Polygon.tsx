import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { Button, Divider } from '@mui/material'
import useDeepCompareEffect from 'use-deep-compare-effect'
import { Popup } from 'react-leaflet'
import type { Feature } from 'geojson'

import { useStatic } from '@hooks/useStatic'
import ExportPolygon from '@components/interface/dialogs/Polygon'
import { getData } from '@services/fetches'
import { useShapes } from '@hooks/useShapes'

export default function PolygonPopup({ feature: ref }: { feature: Feature }) {
  const layerEditing = useStatic((s) => s.layerEditing)
  const feature = useShapes((s) =>
    ref.geometry.type === 'Polygon'
      ? s.Polygon[ref.id as number | string]
      : s.MultiPolygon[ref.id as number | string],
  ) || { properties: {}, geometry: { type: 'Polygon', coordinates: [] } }
  const remove = useShapes((s) => s.setters.remove)

  const [open, setOpen] = React.useState('')
  const [active, setActive] = React.useState({
    spawnpoint: 0,
    gym: 0,
    pokestop: 0,
  })

  useDeepCompareEffect(() => {
    if (!feature.geometry.coordinates.length) return
    Promise.all(
      ['pokestop', 'gym', 'spawnpoint'].map((category) =>
        getData<{ total: number }>(`/api/data/area_stats/${category}`, {
          area: feature,
        }).then((data) =>
          setActive((prev) => ({ ...prev, [category]: data?.total ?? 0 })),
        ),
      ),
    )
  }, [feature])

  return feature && Object.values(layerEditing).every((v) => !v) ? (
    <Popup>
      <Grid2 container spacing={2} minWidth={150}>
        <Grid2 xs={12}>{feature.properties?.name}</Grid2>
        <Grid2 xs={12}>{feature.properties?.type}</Grid2>
        <Divider
          flexItem
          sx={{ my: 1, color: 'black', width: '90%', height: 2 }}
        />
        <Grid2 xs={12}>Pokestops: {active.pokestop.toLocaleString()}</Grid2>
        <Grid2 xs={12}>Gyms: {active.gym.toLocaleString()}</Grid2>
        <Grid2 xs={12}>Spawnpoints: {active.spawnpoint.toLocaleString()}</Grid2>
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
    </Popup>
  ) : null
}
