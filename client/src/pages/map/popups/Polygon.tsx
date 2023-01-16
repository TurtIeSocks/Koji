import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import {
  Button,
  ButtonGroup,
  CircularProgress,
  Divider,
  MenuItem,
  Select,
  TextField,
  Typography,
} from '@mui/material'
import useDeepCompareEffect from 'use-deep-compare-effect'
import type { Feature } from 'geojson'

import ExportPolygon from '@components/dialogs/Polygon'
import { getData } from '@services/fetches'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'

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
  const [name, setName] = React.useState(feature.properties?.__name || '')
  const [type, setType] = React.useState(feature.properties?.__type || '')

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
        <Grid2 xs={12} pt={1}>
          <TextField
            label="Name"
            size="small"
            fullWidth
            value={name}
            onChange={({ target }) => setName(target.value)}
          />
        </Grid2>
        <Grid2 xs={12} py={1}>
          <Select
            size="small"
            fullWidth
            value={type}
            onChange={async ({ target }) => setType(target.value)}
          >
            {(useStatic.getState().scannerType === 'rdm'
              ? RDM_FENCES
              : UNOWN_FENCES
            ).map((t) => (
              <MenuItem key={t} value={t}>
                {t}
              </MenuItem>
            ))}
          </Select>
        </Grid2>
        <Divider flexItem sx={{ my: 1, color: 'black', width: '90%' }} />
        <Grid2 xs={12}>
          <Typography variant="subtitle2">
            Pokestops: {getState('pokestop')}
          </Typography>
          <Typography variant="subtitle2">Gyms: {getState('gym')}</Typography>
          <Typography variant="subtitle2">
            Spawnpoints: {getState('spawnpoint')}
          </Typography>
        </Grid2>
        <Divider flexItem sx={{ my: 1, color: 'black', width: '90%' }} />
        <Grid2>
          <Button size="small" onClick={() => setOpen('polygon')}>
            Export Polygon
          </Button>
          <br />
          <Button
            size="small"
            onClick={() => remove(feature.geometry.type, feature.id)}
          >
            Remove From Map
          </Button>
          <br />
          <ButtonGroup>
            <Button
              disabled={feature.properties?.__koji_id === undefined}
              size="small"
              onClick={async () => {
                remove(feature.geometry.type, feature.id)
                await fetch(
                  `/internal/admin/geofence/${feature.properties?.__koji_id}`,
                  {
                    method: 'DELETE',
                  },
                )
              }}
            >
              Delete
            </Button>
            <Button
              disabled={feature.properties?.__name === undefined}
              size="small"
              onClick={() => {
                fetch(
                  feature.properties?.__koji_id
                    ? `/internal/admin/geofence/${feature.properties?.__koji_id}`
                    : '/internal/admin/geofence',
                  {
                    method: feature.properties?.__koji_id ? 'PATCH' : 'POST',
                    headers: {
                      'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                      id: feature.properties?.__koji_id || 0,
                      name,
                      mode: type,
                      area: feature,
                      updated_at: new Date(),
                      created_at: new Date(),
                    }),
                  },
                )
                  .then((res) => res.json())
                  .then((res) => {
                    const newFeature = {
                      ...res.data.area,
                      properties: {
                        ...res.data.properties,
                        __name: res.data.name,
                        __type: res.data.mode,
                        __koji_id: res.data.id,
                      },
                    }
                    useShapes
                      .getState()
                      .setters.remove(feature.geometry.type, feature.id)
                    useShapes.getState().setters.add(newFeature, '__KOJI')
                  })
              }}
            >
              {feature.properties?.__koji_id ? 'Save' : 'Create'}
            </Button>
          </ButtonGroup>
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
