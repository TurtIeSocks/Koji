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
import type { Feature, MultiPoint } from 'geojson'

import { KojiResponse, PopupProps } from '@assets/types'
import ExportRoute from '@components/dialogs/ExportRoute'
import { useShapes } from '@hooks/useShapes'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { RDM_ROUTES, UNOWN_ROUTES } from '@assets/constants'
import { useStatic } from '@hooks/useStatic'

interface Props extends PopupProps {
  id: Feature['id']
  lat: number
  lon: number
  properties: Feature['properties']
  type: 'Point' | 'MultiPoint'
}

export function PointPopup({ id, lat, lon, properties, type: geoType }: Props) {
  const [open, setOpen] = React.useState('')
  const feature = useShapes((s) => s[geoType][id as number | string])
  const { add, remove, splitLine } = useShapes.getState().setters

  const [name, setName] = React.useState(feature.properties?.__name || '')
  const [type, setType] = React.useState(feature.properties?.__type || '')
  const [loading, setLoading] = React.useState(false)

  const removeCheck = () =>
    useShapes.getState().activeRoute === feature.id
      ? remove(feature.geometry.type, feature.id)
      : remove('Point')

  return id !== undefined ? (
    <div>
      {properties?.__name && (
        <>
          Name: {properties.name}
          <br />
        </>
      )}
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
            value={type}
            onChange={async ({ target }) => setType(target.value)}
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
        <Grid2
          xs={6}
          disabled={feature.properties?.backward === undefined}
          component={Button}
          onClick={() => splitLine(`${feature.properties?.backward}_${id}`)}
        >
          <ChevronLeft />
          <Add />
        </Grid2>
        <Grid2
          xs={6}
          disabled={feature.properties?.forward === undefined}
          component={Button}
          onClick={() => splitLine(`${id}_${feature.properties?.forward}`)}
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
              disabled={feature.properties?.__koji_id === undefined}
              onClick={async () => {
                setLoading(true)
                await fetch(
                  `/internal/admin/route/${feature.properties?.__koji_id}/`,
                  {
                    method: 'DELETE',
                  },
                ).then(() => {
                  setLoading(false)
                  removeCheck()
                })
              }}
            >
              Delete
            </Button>
            <Button
              disabled={!name || !type || loading}
              onClick={() => {
                setLoading(true)
                fetch(`/api/v1/convert/merge_points`, {
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
                })
                  .then((res) => res.json())
                  .then((mp: KojiResponse<Feature<MultiPoint>>) =>
                    fetch(
                      feature.properties?.__koji_id
                        ? `/internal/admin/route/${feature.properties?.__koji_id}/`
                        : '/internal/admin/route/',
                      {
                        method: feature.properties?.__koji_id
                          ? 'PATCH'
                          : 'POST',
                        headers: {
                          'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                          id: feature.properties?.__koji_id || 0,
                          name,
                          geofence_id: feature.properties?.__geofence_id,
                          mode: type,
                          geometry: mp.data.geometry,
                          updated_at: new Date(),
                          created_at: new Date(),
                        }),
                      },
                    )
                      .then((res) => res.json())
                      .then((res) => {
                        const newFeature = {
                          ...feature,
                          id: `${res.data.name}__${res.data.mode}__KOJI`,
                          geometry: res.data.geometry,
                          properties: {
                            ...feature.properties,
                            __name: res.data.name,
                            __type: res.data.mode,
                            __koji_id: res.data.id,
                          },
                        }
                        setLoading(false)
                        removeCheck()
                        add(newFeature, '__KOJI')
                      }),
                  )
              }}
            >
              {feature.properties?.__koji_id ? 'Save' : 'Create'}
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
