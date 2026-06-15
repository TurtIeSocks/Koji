import * as React from 'react'
import {
  Button,
  ButtonGroup,
  MenuItem,
  Select,
  SvgIcon,
  TextField,
} from '@mui/material'
import ChevronLeft from '@mui/icons-material/ChevronLeft'
import Add from '@mui/icons-material/Add'
import geohash from 'ngeohash'
import type { MultiPoint } from 'geojson'
import { S2CellId, S2LatLng } from 'nodes2ts'

import { Feature, KojiRoute, PopupProps } from '@assets/types'
import { useShapes } from '@hooks/useShapes'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { UNOWN_ROUTES } from '@assets/constants'
import { useStatic } from '@hooks/useStatic'
import { fetchWrapper, getKojiCache, pollJob } from '@services/fetches'
import { useDbCache } from '@hooks/useDbCache'
import { useImportExport } from '@hooks/useImportExport'
import { usePersist } from '@hooks/usePersist'

const { add, remove, splitLine, activeRoute, updateProperty } =
  useShapes.getState().setters

interface Props extends PopupProps {
  lat: number
  lon: number
  type: 'Point' | 'MultiPoint'
}

export function PointPopup({ id, lat, lon, type: geoType, dbRef }: Props) {
  const feature = useShapes((s) => s[geoType][id])
  const { setRecord, geofence } = useDbCache.getState()
  const s2level = usePersist((s) => s.s2_level)

  const [name, setName] = React.useState(
    dbRef?.name === feature?.properties?.__name
      ? dbRef?.name || feature.properties?.__name || ''
      : feature.properties?.__name || '',
  )
  const [mode, setMode] = React.useState(
    dbRef?.mode || feature.properties?.__mode || '',
  )
  const [fenceId, setFenceId] = React.useState(
    dbRef?.name === feature?.properties?.__name
      ? dbRef?.geofence_id || feature?.properties?.__geofence_id || 0
      : feature?.properties?.__geofence_id || 0,
  )

  const options = Object.values(geofence)

  const [loading, setLoading] = React.useState(false)

  const removeCheck = () =>
    useShapes.getState().activeRoute === feature.id
      ? remove(feature.geometry.type, feature.id)
      : remove('Point')

  const isInKoji = dbRef?.id && dbRef?.name === feature?.properties?.__name
  feature.properties?.__multipoint_id?.toString().endsWith('KOJI')

  const cell = S2CellId.fromPoint(
    S2LatLng.fromDegrees(lon, lat).toPoint(),
  ).parentL(s2level)

  return id !== undefined ? (
    <div>
      Lat: {lat}
      <br />
      Lng: {lon}
      <br />
      {process.env.NODE_ENV === 'development' && (
        <>
          ID: {id}
          <br />
          Hash: {geohash.encode(lat, lon, 9)}
          <br />
          Hash: {geohash.encode(lat, lon, 12)}
          <br />
          S2: {cell.id.toString()} ({cell.face})
        </>
      )}
      <br />
      <Grid2 container>
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
            value={mode}
            onChange={({ target }) => setMode(target.value)}
            onBlur={() =>
              updateProperty(feature.geometry.type, feature.id, '__mode', mode)
            }
          >
            {UNOWN_ROUTES.map((t) => (
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
            value={options.length ? fenceId || '' : ''}
            onChange={({ target }) => setFenceId(+target.value)}
            onOpen={() => (options.length ? null : getKojiCache('geofence'))}
            onBlur={() =>
              updateProperty(
                feature.geometry.type,
                feature.id,
                '__geofence_id',
                fenceId,
              )
            }
          >
            {options.map((t) => (
              <MenuItem key={t.id} value={t.id}>
                {t.name}
              </MenuItem>
            ))}
          </Select>
        </Grid2>
        <Grid2
          xs={6}
          disabled={feature.properties?.__backward === undefined}
          component={Button}
          onClick={() => splitLine(`${feature.properties?.__backward}__${id}`)}
        >
          <ChevronLeft />
          <Add />
        </Grid2>
        <Grid2
          xs={6}
          disabled={feature.properties?.__forward === undefined}
          component={Button}
          onClick={() => splitLine(`${id}__${feature.properties?.__forward}`)}
        >
          <Add />
          <SvgIcon>
            {/* Chevron right import seems to be broken... */}
            <path d="M10 6 8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
          </SvgIcon>
        </Grid2>
        <Grid2
          xs={12}
          my={1}
          component={Button}
          onClick={() => remove('Point', id)}
        >
          Remove
        </Grid2>
        <Grid2 xs={12} my={1} component={Button} onClick={() => removeCheck()}>
          Remove All
        </Grid2>
        <Grid2
          xs={12}
          component={Button}
          onClick={() =>
            useImportExport.setState({
              open: 'exportRoute',
              feature:
                typeof id === 'string'
                  ? useShapes.getState().MultiPoint[
                      feature.properties.__multipoint_id || ''
                    ]
                  : useShapes.getState().getters.getPointsAsMp(),
            })
          }
        >
          Export Route
        </Grid2>
        <Grid2
          xs={12}
          component={Button}
          disabled={loading}
          onClick={async () => {
            setLoading(true)
            const {
              route_split_level,
              save_to_scanner,
              save_to_db,
              sort_by,
              routing_args,
            } = usePersist.getState()
            const { setStatic } = useStatic.getState()
            setStatic('loading', { [name]: null })
            setStatic('totalLoadingTime', 0)
            setStatic('totalStartTime', Date.now())
            const start = Date.now()
            // v2 reroute: a `mode: 'reroute'` calc job. POST /api/v2/jobs →
            // {job_id} → pollJob → result.data (FeatureCollection, take [0]) +
            // result.stats. (v1 was the sync /api/v1/calc/reroute.)
            // TODO(v2-verify): the reroute job path is runtime-unverified; confirm
            // the result FC carries the routed MultiPoint feature at [0].
            const enqueue = await fetchWrapper<{ job_id: string }>(
              `/api/v2/jobs`,
              {
                method: 'POST',
                headers: {
                  'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                  mode: 'reroute',
                  clusters: Object.values(useShapes.getState().Point).map(
                    (p) => [
                      p.geometry.coordinates[1],
                      p.geometry.coordinates[0],
                    ],
                  ),
                  instance: name,
                  routing: {
                    sortBy: sort_by,
                    routeSplitLevel: route_split_level,
                    pluginArgs: routing_args || undefined,
                  },
                  output: { returnType: 'feature', saveToScanner: save_to_scanner },
                }),
              },
            )
            const record = enqueue?.job_id
              ? await pollJob(enqueue.job_id).catch(() => null)
              : null
            await Promise.resolve().then(() => {
              const res = record?.result
              if (record?.status === 'succeeded' && res) {
                if (save_to_db) {
                  useStatic.setState({
                    notification: {
                      message: 'Saved successfully!',
                      status: 200,
                      severity: 'success',
                    },
                  })
                }
                const end = Date.now() - start
                if (res.stats) {
                  setStatic('loading', (prev) => ({
                    ...prev,
                    [name]: {
                      ...res.stats,
                      fetch_time: end,
                    },
                  }))
                }
                const rerouted = res.data?.features?.[0]
                const newFeature = {
                  ...feature,
                  ...rerouted,
                  id:
                    feature.properties?.__multipoint_id ||
                    feature.id.toString(),
                  properties: { ...rerouted?.properties, ...feature.properties },
                }
                setStatic('totalLoadingTime', end)
                removeCheck()
                activeRoute()
                add(newFeature)
              }
              setLoading(false)
            })
          }}
        >
          Reroute
        </Grid2>
        <Grid2 xs={12}>
          <ButtonGroup>
            <Button
              disabled={!isInKoji}
              onClick={async () => {
                setLoading(true)
                // v2 `DELETE /api/v2/routes/{id}` → 204 (fetchWrapper returns null).
                await fetchWrapper(`/api/v2/routes/${dbRef?.id}`, {
                  method: 'DELETE',
                }).then(() => {
                  setLoading(false)
                  removeCheck()
                  activeRoute()
                })
              }}
            >
              Delete
            </Button>
            <Button
              disabled={!name || !mode || loading || !fenceId}
              onClick={() => {
                setLoading(true)
                // v2 `POST /api/v2/geometry/merge-points?format=feature` →
                // a single merged MultiPoint Feature (enveloped; fetchWrapper
                // unwraps to the Feature directly).
                fetchWrapper<Feature<MultiPoint>>(
                  `/api/v2/geometry/merge-points?format=feature`,
                  {
                    method: 'POST',
                    headers: {
                      'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                      area: {
                        type: 'FeatureCollection',
                        features: Object.values(useShapes.getState().Point),
                      },
                    }),
                  },
                ).then(
                  (mp) =>
                    mp &&
                    // v2 route CRUD: PATCH /api/v2/routes/{id} | POST /api/v2/routes.
                    // The create body needs geofence_id + name + geometry (snake);
                    // PATCH takes the partial. Returns the upserted record (enveloped
                    // → unwrapped to the record).
                    fetchWrapper<KojiRoute>(
                      isInKoji
                        ? `/api/v2/routes/${dbRef?.id}`
                        : '/api/v2/routes',
                      {
                        method: isInKoji ? 'PATCH' : 'POST',
                        headers: {
                          'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                          name,
                          geofence_id: fenceId,
                          mode,
                          geometry: mp.geometry,
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
                        const { geometry, ...rest } = res
                        const newId = `${rest.id}__${rest.mode}__KOJI` as const
                        const newFeature = {
                          ...feature,
                          id: newId,
                          geometry,
                        }
                        setRecord('route', rest.id, {
                          ...rest,
                          geo_type: geometry.type,
                        })
                        setRecord('feature', newId, newFeature)

                        removeCheck()
                        activeRoute()
                        add(newFeature)
                      }
                      setLoading(false)
                    }),
                )
              }}
            >
              {isInKoji ? 'Save' : 'Create'}
            </Button>
          </ButtonGroup>
        </Grid2>
      </Grid2>
    </div>
  ) : null
}

export const MemoPointPopup = React.memo(PointPopup, () => true)
