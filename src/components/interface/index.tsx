import * as React from 'react'
import { useMap, ZoomControl } from 'react-leaflet'
import useButtons from '@hooks/useButtons'

import SelectInstance from './dialogs/Instance'
import Locate from './Locate'

export default function Interface() {
  const map = useMap()

  useButtons(map)

  return (
    <>
      <Locate map={map} />
      <ZoomControl position="bottomright" />
      <SelectInstance />
    </>
  )
}
