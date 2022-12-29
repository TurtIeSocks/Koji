import * as React from 'react'
import geohash from 'ngeohash'
import { PopupProps } from '@assets/types'

interface Props extends PopupProps {
  lat: number
  lon: number
}

export default function PointPopup({ lat, lon }: Props) {
  return (
    <div>
      Lat: {lat.toFixed(6)}
      <br />
      Lng: {lon.toFixed(6)}
      <br />
      Hash: {geohash.encode(lat, lon, 9)}
      <br />
      Hash: {geohash.encode(lat, lon, 12)}
    </div>
  )
}
