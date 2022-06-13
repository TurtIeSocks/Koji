import React from 'react'
import { renderToString } from 'react-dom/server'
import { useMap, ZoomControl } from 'react-leaflet'
import L from 'leaflet'
import { Search } from '@mui/icons-material'

import { useStatic } from '@hooks/useStore'

import Locate from './Locate'
import SelectInstance from './dialogs/Instance'

export default function Interface() {
  const setOpen = useStatic(s => s.setOpen)

  const map = useMap()

  const CustomControlSearch = L.Control.extend({
    options: {
      position: 'topright',
    },
    onAdd: () => {
      const container = L.DomUtil.create(
        'div',
        'leaflet-bar leaflet-control leaflet-control-custom',
      )
      container.innerHTML = renderToString(
        <a href="#">
          <Search sx={{ p: 20 }} />
        </a>,
      )

      container.onclick = () => {
        setOpen('instance')
      }
      return container
    },
  })

  React.useEffect(() => {
    map.addControl(new CustomControlSearch())
  }, [])

  return (
    <>
      <Locate map={map} />
      <ZoomControl position="bottomright" />
      <SelectInstance setOpen={setOpen} />
    </>
  )
}
