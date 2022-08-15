import React from 'react'
import { Drawer, Box, List, Divider } from '@mui/material'
import { ChevronRight } from '@mui/icons-material'

import { useStore, type UseStore } from '@hooks/useStore'

import DrawerHeader from '../styled/DrawerHeader'
import InstanceSelect from './Instance'
import NumInput from './NumInput'
import Mode from './Mode'

interface Props {
  drawer: boolean
  setDrawer: (drawer: boolean) => void
  drawerWidth: number
}

export default function DrawerIndex({ drawer, setDrawer, drawerWidth }: Props) {
  const apiSettings = useStore((s) => s.apiSettings)
  const setApiSettings = useStore((s) => s.setApiSettings)

  const toggleDrawer = (event: React.KeyboardEvent | React.MouseEvent) => {
    if (
      event &&
      event.type === 'keydown' &&
      ((event as React.KeyboardEvent).key === 'Tab' ||
        (event as React.KeyboardEvent).key === 'Shift')
    ) {
      return
    }
    setDrawer(false)
  }

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.code === 'Escape') {
      e.preventDefault()
      setDrawer(false)
    }
  }

  const handleApiChange = (
    field: keyof UseStore['apiSettings'],
    value: string | number,
  ) => {
    setApiSettings({ ...apiSettings, [field]: value })
  }

  React.useEffect(() => {
    window.addEventListener('keydown', handleKeyPress)
    return () => window.removeEventListener('keydown', handleKeyPress)
  }, [])

  return (
    <Drawer
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width: drawerWidth,
        },
      }}
      variant="persistent"
      anchor="left"
      open
      onClose={toggleDrawer}
    >
      {drawer ? (
        <>
          <DrawerHeader setDrawer={setDrawer}>Kōji</DrawerHeader>
          <List>
            <InstanceSelect
              value={apiSettings.instance}
              setValue={handleApiChange}
            />
            <Divider sx={{ my: 2 }} />
            <NumInput
              field="radius"
              value={apiSettings.radius}
              setValue={handleApiChange}
            />
            <NumInput
              field="generations"
              value={apiSettings.generations}
              setValue={handleApiChange}
            />
            <Divider sx={{ my: 2 }} />
            <Mode value={apiSettings.mode} setValue={handleApiChange} />
            <Divider sx={{ my: 2 }} />
          </List>
        </>
      ) : (
        <Box
          sx={{
            width: '100%',
            height: '100vh',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: '0.50s ease',
            '&:hover': {
              backgroundColor: '#cfcfcf',
            },
          }}
          onClick={() => setDrawer(true)}
        >
          <ChevronRight fontSize="small" />
        </Box>
      )}
    </Drawer>
  )
}
