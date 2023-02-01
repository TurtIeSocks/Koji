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

import { Feature, KojiResponse, KojiRoute, PopupProps } from '@assets/types'
import ExportRoute from '@components/dialogs/ExportRoute'
import { useShapes } from '@hooks/useShapes'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { RDM_ROUTES, UNOWN_ROUTES } from '@assets/constants'
import { useStatic } from '@hooks/useStatic'
import { getData, getKojiCache } from '@services/fetches'
import { useDbCache } from '@hooks/useDbCache'

interface Props extends PopupProps {
  lat: number
  lon: number
  type: 'Point' | 'MultiPoint'
}

export function PointPopup({ id, lat, lon, type: geoType, dbRef }: Props) {
  const [open, setOpen] = React.useState('')
  const feature = useShapes((s) => s[geoType][id])
  const { add, remove, splitLine, activeRoute } = useShapes.getState().setters
  const { setRecord, geofence } = useDbCache.getState()

  const [name, setName] = React.useState(
    dbRef?.name || feature.properties?.__name || '',
  )
  const [mode, setMode] = React.useState(
    dbRef?.mode || feature.properties?.__mode || '',
  )
  const [fenceId, setFenceId] = React.useState(dbRef?.geofence_id || 0)
  const options = Object.values(geofence)

  const [loading, setLoading] = React.useState(false)

  const removeCheck = () =>
    useShapes.getState().activeRoute === feature.id
      ? remove(feature.geometry.type, feature.id)
      : remove('Point')

  const isInKoji = feature.properties?.__multipoint_id?.endsWith('KOJI')

  return id !== undefined ? (
    <div>
      Lat: {lat.toFixed(6)}
      <br />
      Lng: {lon.toFixed(6)}
      <br />
      {process.env.NODE_ENV === 'development' && (
        <>
          ID: {id}
          <br />
          Hash: {geohash.encode(lat, lon, 9)}
          <br />
          Hash: {geohash.encode(lat, lon, 12)}
          <br />
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
          />
        </Grid2>
        <Grid2 xs={12} py={1}>
          <Select
            size="small"
            fullWidth
            value={mode}
            onChange={({ target }) => setMode(target.value)}
          >
            {(useStatic.getState().scannerType === 'rdm'
              ? RDM_ROUTES
              : UNOWN_ROUTES
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
            value={options.length ? fenceId || '' : ''}
            onChange={({ target }) => setFenceId(+target.value)}
            onOpen={async () =>
              options.length ? null : getKojiCache('geofence')
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
        <Grid2 xs={12} component={Button} onClick={() => setOpen('route')}>
          Export Route
        </Grid2>
        <Grid2 xs={12}>
          <ButtonGroup>
            <Button
              disabled={!isInKoji}
              onClick={async () => {
                setLoading(true)
                await getData(`/internal/admin/route/${dbRef?.id}/`, {
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
                getData<KojiResponse<Feature<MultiPoint>>>(
                  `/api/v1/convert/merge_points`,
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
                      return_type: 'feature',
                    }),
                  },
                ).then(
                  (mp) =>
                    mp &&
                    getData<KojiResponse<KojiRoute>>(
                      isInKoji
                        ? `/internal/admin/route/${dbRef?.id}/`
                        : '/internal/admin/route/',
                      {
                        method: isInKoji ? 'PATCH' : 'POST',
                        headers: {
                          'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                          id: isInKoji ? dbRef?.id : 0,
                          name,
                          geofence_id: fenceId,
                          mode,
                          geometry: mp.data.geometry,
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
                        const { geometry, ...rest } = res.data
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
      {open && (
        <ExportRoute
          open={open}
          setOpen={setOpen}
          geojson={{
            type: 'FeatureCollection',
            features:
              typeof id === 'string'
                ? [useShapes.getState().MultiPoint[id?.split('___')[0]]]
                : Object.values(useShapes.getState().Point),
          }}
        />
      )}
    </div>
  ) : null
}

export const MemoPointPopup = React.memo(PointPopup, () => true)
