import * as React from 'react'
import { ZoomControl } from 'react-leaflet'

import Locate from './Locate'

export default function Interface() {
  return (
    <>
      <Locate />
      <ZoomControl position="bottomright" />
    </>
  )
}
