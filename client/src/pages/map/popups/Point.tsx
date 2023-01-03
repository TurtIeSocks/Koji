import * as React from 'react'
import { Button, SvgIcon } from '@mui/material'
import ChevronLeft from '@mui/icons-material/ChevronLeft'
import Add from '@mui/icons-material/Add'
import geohash from 'ngeohash'
import type { Feature } from 'geojson'

import { PopupProps } from '@assets/types'
import ExportRoute from '@components/dialogs/ExportRoute'
import { useShapes } from '@hooks/useShapes'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

interface Props extends PopupProps {
  id: Feature['id']
  lat: number
  lon: number
  properties: Feature['properties']
}

export function PointPopup({ id, lat, lon, properties }: Props) {
  const [open, setOpen] = React.useState('')

  return id !== undefined ? (
    <div>
      {properties?.name && (
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
        <Grid2
          xs={6}
          disabled={
            useShapes.getState().Point[id]?.properties?.backward === undefined
          }
          component={Button}
          onClick={() =>
            useShapes
              .getState()
              .setters.splitLine(
                `${useShapes.getState().Point[id]?.properties?.backward}_${id}`,
              )
          }
        >
          <ChevronLeft />
          <Add />
        </Grid2>
        <Grid2
          xs={6}
          disabled={
            useShapes.getState().Point[id]?.properties?.forward === undefined
          }
          component={Button}
          onClick={() =>
            useShapes
              .getState()
              .setters.splitLine(
                `${id}_${useShapes.getState().Point[id]?.properties?.forward}`,
              )
          }
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
          onClick={() => useShapes.getState().setters.remove('Point', id)}
        >
          Remove
        </Grid2>
        <Grid2 xs={12} component={Button} onClick={() => setOpen('route')}>
          Export Route
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
