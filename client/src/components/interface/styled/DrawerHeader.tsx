import * as React from 'react'
import { Typography, IconButton, styled } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import { ChevronLeft } from '@mui/icons-material'
import { UseStore } from '@hooks/useStore'

const DrawerHeaderRaw = styled(Grid2)(() => ({
  display: 'flex',
  alignItems: 'center',
  minHeight: 56,
  justifyContent: 'flex-end',
}))

interface Props {
  setStore: UseStore['setStore']
  children: React.ReactNode
}

export default function DrawerHeader({ setStore, children }: Props) {
  return (
    <DrawerHeaderRaw container>
      <Grid2 xs={10}>
        <Typography variant="h5" align="center">
          {children}
        </Typography>
      </Grid2>
      <Grid2 xs={2}>
        <IconButton onClick={() => setStore('drawer', false)}>
          <ChevronLeft />
        </IconButton>
      </Grid2>
    </DrawerHeaderRaw>
  )
}
