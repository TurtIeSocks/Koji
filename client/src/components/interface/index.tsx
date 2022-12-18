import * as React from 'react'
import { ZoomControl } from 'react-leaflet'

// import Drawing from '@components/markers/Drawing'
import useCluster from '@hooks/useCluster'
import useLayers from '@hooks/useLayers'

import Locate from './Locate'

export default function Interface() {
  useCluster()
  useLayers()

  return (
    <>
      <Locate />
      <ZoomControl position="bottomright" />
      {/* <Drawing /> */}
    </>
  )
}
