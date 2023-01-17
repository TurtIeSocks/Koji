import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import {
  Button,
  ButtonGroup,
  CircularProgress,
  Divider,
  Menu,
  MenuItem,
  Select,
  TextField,
  Typography,
} from '@mui/material'
import useDeepCompareEffect from 'use-deep-compare-effect'
import type { Feature, MultiPolygon, Polygon } from 'geojson'

import ExportPolygon from '@components/dialogs/Polygon'
import { getData } from '@services/fetches'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'
import {
  removeAllOthers,
  removeThisPolygon,
  splitMultiPolygons,
} from '@services/utils'

export function PolygonPopup({
  feature: ref,
  loadData,
}: {
  feature: Feature<Polygon | MultiPolygon>
  loadData: boolean
}) {
  const feature = useShapes((s) =>
    ref.geometry.type === 'Polygon'
      ? s.Polygon[ref.id as number | string]
      : s.MultiPolygon[ref.id as number | string],
  ) ||
    ref || { properties: {}, geometry: { type: 'Polygon', coordinates: [] } }
  const { add, remove } = useShapes.getState().setters

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

  const [mapAnchorEl, setMapAnchorEl] = React.useState<null | HTMLElement>(null)
  const [dbAnchorEl, setDbAnchorEl] = React.useState<null | HTMLElement>(null)

  const handleClick =
    (map = false) =>
    (event: React.MouseEvent<HTMLButtonElement>) => {
      if (map) {
        setMapAnchorEl(event.currentTarget)
      } else {
        setDbAnchorEl(event.currentTarget)
      }
    }

  const handleClose = () => {
    setMapAnchorEl(null)
    setDbAnchorEl(null)
  }

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
    <React.Fragment key={feature.geometry.type}>
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
        <Grid2 xs={12}>
          <Typography variant="subtitle2">
            {feature.geometry.type}
            {feature.geometry.type === 'MultiPolygon'
              ? `(${feature.geometry.coordinates.length})`
              : ''}
          </Typography>
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
        <Grid2 xs={12}>
          <Typography variant="h6" gutterBottom>
            Action Menus
          </Typography>
          <ButtonGroup>
            <Button onClick={handleClick(true)}>Map</Button>
            <Button onClick={handleClick(false)}>Database</Button>
          </ButtonGroup>
        </Grid2>
      </Grid2>
      <Menu anchorEl={mapAnchorEl} open={!!mapAnchorEl} onClose={handleClose}>
        <MenuItem
          onClick={() => {
            setOpen('polygon')
            handleClose()
          }}
        >
          Export
        </MenuItem>
        <MenuItem
          onClick={() => {
            remove(feature.geometry.type, feature.id)
            handleClose()
          }}
        >
          Remove
        </MenuItem>
        {...feature.geometry.type === 'MultiPolygon'
          ? [
              <MenuItem
                onClick={() => {
                  const split = splitMultiPolygons({
                    type: 'FeatureCollection',
                    features: [feature],
                  })
                  remove(feature.geometry.type, feature.id)
                  add(split.features)
                  handleClose()
                }}
              >
                Split into Polygons
              </MenuItem>,
              <MenuItem
                onClick={() => {
                  removeThisPolygon(feature as Feature<MultiPolygon>)
                  handleClose()
                }}
              >
                Remove this Polygon
              </MenuItem>,
              <MenuItem
                onClick={() => {
                  removeAllOthers(feature as Feature<MultiPolygon>)
                  handleClose()
                }}
              >
                Remove Others
              </MenuItem>,
            ]
          : [
              <MenuItem
                onClick={() => {
                  remove(feature.geometry.type, feature.id)
                  add({
                    ...feature,
                    geometry: {
                      ...feature.geometry,
                      type: 'MultiPolygon',
                      coordinates: [
                        feature.geometry.coordinates as Polygon['coordinates'],
                      ],
                    },
                  })
                  handleClose()
                }}
              >
                To MultiPolygon
              </MenuItem>,
            ]}
      </Menu>
      <Menu anchorEl={dbAnchorEl} open={!!dbAnchorEl} onClose={handleClose}>
        <MenuItem
          disabled={feature.properties?.__name === undefined}
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
                handleClose()
              })
          }}
        >
          {feature.properties?.__koji_id ? 'Save' : 'Create'}
        </MenuItem>
        <MenuItem
          disabled={feature.properties?.__koji_id === undefined}
          onClick={async () => {
            remove(feature.geometry.type, feature.id)
            await fetch(
              `/internal/admin/geofence/${feature.properties?.__koji_id}`,
              {
                method: 'DELETE',
              },
            ).then(() => {
              handleClose()
            })
          }}
        >
          Delete
        </MenuItem>
      </Menu>

      {open && (
        <ExportPolygon
          mode="export"
          open={open}
          setOpen={setOpen}
          feature={feature}
        />
      )}
    </React.Fragment>
  ) : null
}

export const MemoPolyPopup = React.memo(
  PolygonPopup,
  (prev, next) =>
    prev.loadData === next.loadData &&
    prev.feature.geometry.type === next.feature.geometry.type &&
    prev.feature.geometry.coordinates.length ===
      next.feature.geometry.coordinates.length,
)
