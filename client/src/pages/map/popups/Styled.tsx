import * as React from 'react'
import { Paper } from '@mui/material'
import { Popup } from 'react-leaflet'

export default function StyledPopup({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <Popup autoPan={false} autoClose={false}>
      <Paper sx={{ borderRadius: 3 }} elevation={0}>
        {children}
      </Paper>
    </Popup>
  )
}
