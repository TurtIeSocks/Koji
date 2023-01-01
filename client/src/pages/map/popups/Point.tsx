import * as React from 'react'
import geohash from 'ngeohash'
import { PopupProps } from '@assets/types'
import { Button } from '@mui/material'
import ExportRoute from '@components/dialogs/ExportRoute'
import type { Feature } from 'geojson'
import { useShapes } from '@hooks/useShapes'

interface Props extends PopupProps {
  id: Feature['id']
  lat: number
  lon: number
  properties: Feature['properties']
}

export function PointPopup({ id, lat, lon, properties }: Props) {
  const [open, setOpen] = React.useState('')

  return (
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
          Hash: {geohash.encode(lat, lon, 9)}
          <br />
          Hash: {geohash.encode(lat, lon, 12)}
          <br />
        </>
      )}
      <Button onClick={() => setOpen('route')}>Export Route</Button>
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
  )
}

export const MemoPointPopup = React.memo(PointPopup, () => true)
