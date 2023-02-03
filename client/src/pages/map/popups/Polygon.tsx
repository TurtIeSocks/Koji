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
import type { MultiPolygon, Polygon } from 'geojson'
import KeyboardArrowDownIcon from '@mui/icons-material/KeyboardArrowDown'

import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'
import type {
  KojiGeofence,
  KojiResponse,
  Feature,
  DbOption,
  KojiKey,
  KojiModes,
} from '@assets/types'
import ExportPolygon from '@components/dialogs/Polygon'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { useDbCache } from '@hooks/useDbCache'
import { fetchWrapper, save } from '@services/fetches'
import {
  removeAllOthers,
  removeThisPolygon,
  splitMultiPolygons,
} from '@services/utils'
import shallow from 'zustand/shallow'

const { add, remove, updateProperty } = useShapes.getState().setters
const { setRecord } = useDbCache.getState()

export function PolygonPopup({
  feature: refFeature,
  loadData,
  dbRef,
}: {
  feature: Feature<Polygon | MultiPolygon>
  loadData: boolean
  dbRef: DbOption | null
}) {
  const feature =
    useShapes(
      (s) => ({ ...s.Polygon, ...s.MultiPolygon }[refFeature.id]),
      shallow,
    ) || refFeature

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
  const [name, setName] = React.useState(
    dbRef?.name || feature.properties?.__name || '',
  )
  const [mode, setMode] = React.useState<KojiModes | ''>(
    dbRef?.mode || feature.properties?.__mode || '',
  )

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
          fetchWrapper<{ total: number }>(
            `/internal/data/area_stats/${category}`,
            {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify({ area: feature }),
            },
          ).then((data) =>
            setActive((prev) => ({
              ...prev,
              [category]: data?.total ?? (data || 0),
            })),
          ),
        ),
      )
    }
  }, [feature, loadData])

  const isKoji = feature.id.endsWith('KOJI')
  const isScanner = feature.id.endsWith('SCANNER')

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
            onBlur={() =>
              updateProperty(feature.geometry.type, feature.id, '__name', name)
            }
          />
        </Grid2>
        <Grid2 xs={12} py={1}>
          <Select
            size="small"
            fullWidth
            value={mode === 'Unset' ? '' : mode || ''}
            onChange={async ({ target }) => setMode(target.value as KojiModes)}
            onBlur={() =>
              updateProperty(feature.geometry.type, feature.id, '__mode', mode)
            }
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
            <Button
              size="small"
              onClick={handleClick(true)}
              endIcon={<KeyboardArrowDownIcon />}
            >
              Map
            </Button>
            <Button
              size="small"
              onClick={handleClick(false)}
              endIcon={<KeyboardArrowDownIcon />}
            >
              Database
            </Button>
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
          disabled={name === undefined}
          onClick={() => {
            fetchWrapper<KojiResponse<KojiGeofence>>(
              isKoji
                ? `/internal/admin/geofence/${dbRef?.id}/`
                : '/internal/admin/geofence/',
              {
                method: isKoji ? 'PATCH' : 'POST',
                headers: {
                  'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                  id: isKoji ? dbRef?.id : 0,
                  name,
                  mode,
                  area: feature,
                  updated_at: new Date(),
                  created_at: new Date(),
                }),
              },
            ).then((res) => {
              if (res) {
                useStatic.setState({
                  networkStatus: {
                    message: 'Saved successfully!',
                    status: 200,
                    severity: 'success',
                  },
                })
                const { area, mode: newMode, ...rest } = res.data
                if (area.geometry.type !== 'GeometryCollection') {
                  const newId = `${rest.id}__${
                    newMode || 'Unset'
                  }__KOJI` as const
                  area.id = newId
                  setRecord('geofence', rest.id, {
                    ...rest,
                    mode: newMode || 'Unset',
                    geo_type: area.geometry.type,
                  })
                  setRecord('feature', newId, area)
                  remove(feature.geometry.type, feature.id)
                  add(area)
                }
              }
              handleClose()
            })
          }}
        >
          {isKoji ? 'Update Kōji' : 'Save to Kōji'}
        </MenuItem>
        <MenuItem
          disabled={!isKoji}
          onClick={async () => {
            remove(feature.geometry.type, feature.id)
            await fetchWrapper(`/internal/admin/geofence/${dbRef?.id}/`, {
              method: 'DELETE',
            }).then(() => {
              handleClose()
            })
          }}
        >
          Delete from Kōji
        </MenuItem>
        <MenuItem
          disabled={name === undefined}
          onClick={async () => {
            await save(
              '/api/v1/geofence/save-scanner',
              JSON.stringify({
                ...feature,
                properties: {
                  ...feature.properties,
                  __id: isScanner ? dbRef?.id : undefined,
                },
              }),
            ).then((res) => {
              if (res) {
                if (dbRef && mode) {
                  setRecord('scanner', feature.id as KojiKey, {
                    ...dbRef,
                    mode,
                    geo_type: feature.geometry.type,
                  })
                  setRecord('feature', feature.id as KojiKey, feature)
                }
                remove(refFeature.geometry.type, feature.id)
                add(feature)
              }

              handleClose()
            })
          }}
        >
          Update Scanner
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
