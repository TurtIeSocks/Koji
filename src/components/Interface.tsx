import React from 'react'
import { useMap, ZoomControl } from 'react-leaflet'

import Locate from './Locate'

export default function Interface() {
  const map = useMap()
  return (
    <>
      <Locate map={map} />
      <ZoomControl position="bottomright" />
    </>
  )
}
