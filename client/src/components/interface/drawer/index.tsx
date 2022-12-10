import React, { Fragment } from 'react'
import { Box, List, Divider } from '@mui/material'
import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs'
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider'
import type {} from '@mui/x-date-pickers/themeAugmentation'

import { TABS } from '@assets/constants'
import { useStore } from '@hooks/useStore'

import { Drawer } from '../styled/Drawer'
import DrawerHeader from '../styled/DrawerHeader'
import GeofenceTab from './geofence'
import RoutingTab from './routing'
import MenuAccordion from './MenuItem'
import ImportExport from './manage'
import Settings from './settings'
import MiniItem from './MiniItem'

export default function DrawerIndex() {
  const setStore = useStore((s) => s.setStore)
  const drawer = useStore((s) => s.drawer)

  const toggleDrawer = (event: React.KeyboardEvent | React.MouseEvent) => {
    if (
      event &&
      event.type === 'keydown' &&
      ((event as React.KeyboardEvent).key === 'Tab' ||
        (event as React.KeyboardEvent).key === 'Shift')
    ) {
      return
    }
    setStore('drawer', false)
  }

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.code === 'Escape') {
      e.preventDefault()
      setStore('drawer', false)
    }
  }

  React.useEffect(() => {
    window.addEventListener('keydown', handleKeyPress)
    return () => window.removeEventListener('keydown', handleKeyPress)
  }, [])

  return (
    <Drawer variant="permanent" open={drawer} onClose={toggleDrawer}>
      {drawer ? (
        <LocalizationProvider dateAdapter={AdapterDayjs}>
          <DrawerHeader setStore={setStore}>Kōji</DrawerHeader>
          <Divider />
          <List>
            {TABS.map((text, i) => (
              <Fragment key={text}>
                {!!i && <Divider />}
                <MenuAccordion name={text}>
                  {{
                    Drawing: <GeofenceTab />,
                    Clustering: <RoutingTab />,
                    Manage: <ImportExport />,
                    Settings: <Settings />,
                  }[text] || null}
                </MenuAccordion>
              </Fragment>
            ))}
          </List>
        </LocalizationProvider>
      ) : (
        <Box
          sx={{
            width: '100%',
            height: '100vh',
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'center',
          }}
        >
          <List>
            {TABS.map((text, i) => (
              <MiniItem key={text} text={text} i={i} />
            ))}
          </List>
        </Box>
      )}
    </Drawer>
  )
}
