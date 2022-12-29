import * as React from 'react'
import { ZoomControl, useMapEvents } from 'react-leaflet'
import { renderToString } from 'react-dom/server'
import AdminPanelSettings from '@mui/icons-material/AdminPanelSettings'
import { useNavigate } from 'react-router'

import useCluster from '@hooks/useCluster'
import useLayers from '@hooks/useLayers'
import usePopupStyle from '@hooks/usePopupStyle'
import useSyncGeojson from '@hooks/useSyncGeojson'
import { useStatic } from '@hooks/useStatic'

import Locate from './Locate'
import MemoizedDrawing from './Drawing'
import EasyButton from './EasyButton'

export default function Interface() {
  const navigate = useNavigate()

  useCluster()
  useLayers()
  usePopupStyle()
  useSyncGeojson()

  useMapEvents({
    popupopen(e) {
      const isEditing = Object.values(useStatic.getState().layerEditing).some(
        (v) => v,
      )
      if (isEditing) {
        e.popup.close()
      }
    },
  })

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
            icon: renderToString(
              <AdminPanelSettings sx={{ height: '100%', width: '100%' }} />,
            ),
            onClick() {
              navigate('/admin')
            },
          },
        ]}
      />
    </>
  )
}
