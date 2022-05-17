import { Instance } from '@assets/types'
import { getData } from '@services/utils'
import React, { useEffect } from 'react'
import { useMap, ZoomControl } from 'react-leaflet'

import Locate from './Locate'

export default function Interface() {
  const [instances, setInstances] = React.useState<Instance[]>([])
  const map = useMap()

  useEffect(() => {
    getData<Instance[]>('/instances').then((res) => setInstances(res))
  }, [])

  return (
    <>
      <Locate map={map} />
      <ZoomControl position="bottomright" />
    </>
  )
}
