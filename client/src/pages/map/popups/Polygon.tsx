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
  capitalize,
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
  KojiModes,
} from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { useDbCache } from '@hooks/useDbCache'
import { clusteringRouting, fetchWrapper } from '@services/fetches'
import {
  removeAllOthers,
  removeThisPolygon,
  splitMultiPolygons,
} from '@services/utils'
import { shallow } from 'zustand/shallow'
import { useImportExport } from '@hooks/useImportExport'
import { filterPoints, filterPolys } from '@services/geoUtils'
import { usePersist } from '@hooks/usePersist'

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
  const [area, setArea] = React.useState(0)

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
      fetchWrapper<KojiResponse<{ area: number }>>('/api/v1/calc/area', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ area: feature }),
      }).then((res) => res && setArea(res.data.area))
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

  const isKoji = feature.id.toString().endsWith('KOJI')
  // const isScanner = feature.id.endsWith('SCANNER')

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
            value={mode === 'unset' ? '' : mode || ''}
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
          <Typography variant="caption">{area.toLocaleString()} m²</Typography>
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
          <Button onClick={() => clusteringRouting({ feature })} size="small">
            Run {capitalize(usePersist.getState().mode)}
          </Button>
        </Grid2>
        <Grid2 xs={12}>
          <Button
            disabled={!isKoji}
            onClick={() =>
              clusteringRouting({ parent: feature.properties.__id || name })
            }
            size="small"
          >
            {capitalize(usePersist.getState().mode)} Children
          </Button>
        </Grid2>
        <Grid2 xs={12}>
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
          dense
          onClick={() => {
            useImportExport.setState({ feature, open: 'exportPolygon' })
            handleClose()
          }}
        >
          Export
        </MenuItem>
        <MenuItem
          dense
          onClick={() => {
            remove(feature.geometry.type, feature.id)
            handleClose()
          }}
        >
          Remove
        </MenuItem>
        <MenuItem dense onClick={() => filterPolys(feature, true)}>
          Remove Intersecting Polys
        </MenuItem>
        <MenuItem dense onClick={() => filterPolys(feature)}>
          Remove Non-Intersecting Polys
        </MenuItem>
        <MenuItem dense onClick={() => filterPoints(feature, true)}>
          Remove Contained Points
        </MenuItem>
        <MenuItem dense onClick={() => filterPoints(feature)}>
          Remove Non-Contained Points
        </MenuItem>
        {...feature.geometry.type === 'MultiPolygon'
          ? [
              <MenuItem
                dense
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
                dense
                onClick={() => {
                  removeThisPolygon(feature as Feature<MultiPolygon>)
                  handleClose()
                }}
              >
                Remove this Polygon
              </MenuItem>,
              <MenuItem
                dense
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
                dense
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
                  geometry: feature.geometry,
                  updated_at: new Date(),
                  created_at: new Date(),
                }),
              },
            ).then((res) => {
              if (res) {
                useStatic.setState({
                  notification: {
                    message: 'Saved successfully!',
                    status: 200,
                    severity: 'success',
                  },
                })
                const { geometry, mode: newMode = 'unset', ...rest } = res.data
                const newId = `${rest.id}__${newMode}__KOJI` as const
                setRecord('geofence', rest.id, {
                  ...rest,
                  mode: newMode,
                })
                remove(feature.geometry.type, feature.id)
                add(
                  {
                    ...feature,
                    id: newId,
                    geometry,
                  },
                  '__KOJI',
                )
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
        {/* <MenuItem
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
        </MenuItem> */}
      </Menu>
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
