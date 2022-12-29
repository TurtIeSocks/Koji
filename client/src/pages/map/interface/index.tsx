import * as React from 'react'
import { ZoomControl } from 'react-leaflet'
import { renderToString } from 'react-dom/server'
import AdminPanelSettings from '@mui/icons-material/AdminPanelSettings'
import { useNavigate } from 'react-router'

import useCluster from '@hooks/useCluster'
import useLayers from '@hooks/useLayers'
import usePopupStyle from '@hooks/usePopupStyle'

import Locate from './Locate'
import MemoizedDrawing from './Drawing'
import EasyButton from './EasyButton'

export default function Interface() {
  useCluster()
  useLayers()
  usePopupStyle()
  const navigate = useNavigate()

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
