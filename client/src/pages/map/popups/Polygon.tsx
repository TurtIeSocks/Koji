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
  styled,
} from '@mui/material'
import RefreshIcon from '@mui/icons-material/Refresh'
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
  Category,
} from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import { useStatic } from '@hooks/useStatic'
import { useDbCache } from '@hooks/useDbCache'
import {
  clusteringRouting,
  fetchWrapper,
  getKojiCache,
} from '@services/fetches'
import {
  removeAllOthers,
  removeThisPolygon,
  splitMultiPolygons,
} from '@services/utils'
import { useImportExport } from '@hooks/useImportExport'
import { filterPoints, filterPolys } from '@services/geoUtils'
import { usePersist } from '@hooks/usePersist'

const { add, remove, updateProperty } = useShapes.getState().setters
const { setRecord } = useDbCache.getState()

export const LoadingButton = styled(Button, {
  shouldForwardProp: (prop) => prop !== 'fetched' && prop !== 'loading',
})<{ fetched?: boolean; loading?: boolean }>(({ fetched, loading }) => ({
  minWidth: 0,
  '.MuiButton-endIcon': {
    display: fetched ? 'inherit' : 'none',
    marginRight: '-2px',
    marginLeft: '8px',
  },
  '.MuiButton-startIcon': {
    display: loading ? 'inherit' : 'none',
    marginRight: '8px',
    marginLeft: '-2px',
  },
}))
LoadingButton.defaultProps = {
  fetched: false,
  loading: false,
  size: 'small',
  disableRipple: true,
  endIcon: <RefreshIcon fontSize="small" />,
  startIcon: <CircularProgress size={20} />,
}

const MemoStat = React.memo(
  ({
    category,
    id,
    area,
  }: {
    category: Category
    id: Feature['id']
    area: number
  }) => {
    const [stats, setStats] = React.useState<number | null>(null)
    const [loading, setLoading] = React.useState(false)
    const tth = usePersist((s) => s.tth)
    const raw = usePersist((s) => s.last_seen)
    const autoLoad = usePersist((s) => s[`${category}MaxAreaAutoCalc`])
    const feature = useShapes((s) => ({ ...s.Polygon, ...s.MultiPolygon }[id]))

    const getStats = React.useCallback(() => {
      setLoading(true)
      setStats((prev) => (prev === null ? 0 : null))
      const last_seen = typeof raw === 'string' ? new Date(raw) : raw
      fetchWrapper<{ total: number }>(`/internal/data/area_stats/${category}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          area: feature,
          last_seen: Math.floor((last_seen?.getTime?.() || 0) / 1000),
          tth,
        }),
      })
        .then((data) => setStats(data?.total ?? 0))
        .finally(() => setLoading(false))
    }, [tth, raw, feature])

    React.useEffect(() => {
      if ((stats !== null || (area > 0 && area < autoLoad)) && !loading) {
        getStats()
      }
    }, [getStats, area, autoLoad])

    return (
      <Typography variant="subtitle2">
        {capitalize(category)}s: {loading ? '' : stats?.toLocaleString() || ''}
        <LoadingButton
          size="small"
          onClick={getStats}
          fetched={stats !== null}
          loading={loading}
        >
          {typeof stats === 'number' || loading ? '' : 'Get'}
        </LoadingButton>
      </Typography>
    )
  },
)

export function PolygonPopup({
  feature: refFeature,
  dbRef,
}: {
  feature: Feature<Polygon | MultiPolygon>
  dbRef: DbOption | null
}) {
  const feature =
    useShapes((s) => ({ ...s.Polygon, ...s.MultiPolygon }[refFeature.id])) ||
    refFeature
  const geofence = useDbCache((s) => s.geofence)

  const [name, setName] = React.useState(
    dbRef?.name ||
      feature.properties?.__name ||
      `${feature?.geometry?.type || ''}${feature?.id ? `-${feature?.id}` : ''}`,
  )
  const [mode, setMode] = React.useState<KojiModes | ''>(
    dbRef?.mode || feature.properties?.__mode || 'unset',
  )
  const [parent, setParent] = React.useState(
    feature.properties?.__parent || undefined,
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

  useDeepCompareEffect(() => {
    if (feature.geometry.coordinates.length) {
      fetchWrapper<KojiResponse<{ area: number }>>('/api/v1/calc/area', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ area: feature }),
      }).then((res) => res && setArea(res.data.area / 1000000))
    }
  }, [feature])

  const isKoji = feature.id.toString().endsWith('KOJI')
  // const isScanner = feature.id.endsWith('SCANNER')

  const options = Object.values(geofence)

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
            {(useStatic.getState().scannerType === 'unown'
              ? UNOWN_FENCES
              : RDM_FENCES
            ).map((t) => (
              <MenuItem key={t} value={t}>
                {t}
              </MenuItem>
            ))}
          </Select>
        </Grid2>
        <Grid2 xs={12} py={1}>
          <Select
            size="small"
            fullWidth
            value={parent || ''}
            onChange={({ target }) => setParent(+target.value || undefined)}
            onOpen={() => (options.length ? null : getKojiCache('geofence'))}
            onBlur={() =>
              updateProperty(
                feature.geometry.type,
                feature.id,
                '__parent',
                parent,
              )
            }
          >
            <MenuItem value={undefined}>None</MenuItem>
            {options.map((t) => (
              <MenuItem key={t.id} value={t.id}>
                {t.name}
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
          <Typography variant="caption">
            {+area.toFixed(2).toLocaleString()} km²
          </Typography>
        </Grid2>
        <Divider flexItem sx={{ my: 1, color: 'black', width: '90%' }} />
        <Grid2 xs={12}>
          <MemoStat category="pokestop" id={refFeature.id} area={area} />
          <MemoStat category="gym" id={refFeature.id} area={area} />
          <MemoStat category="spawnpoint" id={refFeature.id} area={area} />
        </Grid2>
        <Divider flexItem sx={{ my: 1, color: 'black', width: '90%' }} />
        <Grid2 xs={12}>
          <Button onClick={() => clusteringRouting({ feature })} size="small">
            Run Clustering
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
            Cluster Children
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
        <MenuItem dense onClick={() => filterPoints(feature, true, true)}>
          Combine Contained Points
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
                  parent,
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
    prev.feature.geometry.type === next.feature.geometry.type &&
    prev.feature.geometry.coordinates.length ===
      next.feature.geometry.coordinates.length,
)
