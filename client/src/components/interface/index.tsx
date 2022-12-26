import * as React from 'react'
import { ZoomControl } from 'react-leaflet'

import useCluster from '@hooks/useCluster'
import useLayers from '@hooks/useLayers'
import usePopupStyle from '@hooks/usePopupStyle'

import Locate from './Locate'
import MemoizedDrawing from './Drawing'

export default function Interface() {
  useCluster()
  useLayers()
  usePopupStyle()

  return (
    <>
      <Locate />
      <ZoomControl position="bottomright" />
      <MemoizedDrawing />
    </>
  )
}
