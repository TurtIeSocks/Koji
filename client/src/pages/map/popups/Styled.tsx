import * as React from 'react'
import { Paper } from '@mui/material'
import { Popup } from 'react-leaflet'

import { useStatic } from '@hooks/useStatic'

export default function StyledPopup({
  children,
}: {
  children: React.ReactNode
}) {
  const layerEditing = useStatic((s) => s.layerEditing)

  return Object.values(layerEditing).every((v) => !v) ? (
    <Popup autoPan={false} autoClose={false}>
      <Paper sx={{ borderRadius: 3 }} elevation={0}>
        {children}
      </Paper>
    </Popup>
  ) : null
}
