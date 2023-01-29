import * as React from 'react'
import { ZoomControl, useMapEvents } from 'react-leaflet'
import { useNavigate } from 'react-router'

import { useStatic } from '@hooks/useStatic'
import useLayers from '@hooks/useLayers'
import usePopupStyle from '@hooks/usePopupStyle'
import useSyncGeojson from '@hooks/useSyncGeojson'
import { usePersist } from '@hooks/usePersist'

import Locate from './Locate'
import MemoizedDrawing from './Drawing'
import EasyButton from './EasyButton'

export default function Interface() {
  useLayers()
  usePopupStyle()
  useSyncGeojson()
  const navigate = useNavigate()

  const map = useMapEvents({
    popupopen(e) {
      const isEditing = Object.values(useStatic.getState().layerEditing).some(
        (v) => v,
      )
      if (isEditing || useStatic.getState().combinePolyMode) {
        e.popup.close()
      }
    },
  })

  const onMove = React.useCallback(() => {
    usePersist.setState({
      location: Object.values(map.getCenter()) as [number, number],
      zoom: map.getZoom(),
    })
  }, [map])

  React.useEffect(() => {
    map.on('moveend', onMove)
    return () => {
      map.off('moveend', onMove)
    }
  }, [onMove])

  return (
    <>
      <Locate />
      <ZoomControl position="bottomright" />
      <MemoizedDrawing />
      <EasyButton
        position="bottomright"
        states={[
          {
            stateName: 'default',
            title: 'Admin Panel',
            icon: `
            <svg>
              <path d="M17 11c.34 0 .67.04 1 .09V6.27L10.5 3 3 6.27v4.91c0 4.54 3.2 8.79 7.5 9.82.55-.13 1.08-.32 1.6-.55-.69-.98-1.1-2.17-1.1-3.45 0-3.31 2.69-6 6-6z"></path>
              <path d="M17 13c-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4-1.79-4-4-4zm0 1.38c.62 0 1.12.51 1.12 1.12s-.51 1.12-1.12 1.12-1.12-.51-1.12-1.12.5-1.12 1.12-1.12zm0 5.37c-.93 0-1.74-.46-2.24-1.17.05-.72 1.51-1.08 2.24-1.08s2.19.36 2.24 1.08c-.5.71-1.31 1.17-2.24 1.17z"></path>
            </svg>`,
            onClick() {
              navigate('/admin')
            },
          },
        ]}
      />
    </>
  )
}
