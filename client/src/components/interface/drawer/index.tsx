import React from 'react'
import { Box, List, Divider, ListItemButton, Tooltip } from '@mui/material'

import { ICON_MAP, TABS } from '@assets/constants'
import { useStore } from '@hooks/useStore'

import { Drawer } from '../styled/Drawer'
import DrawerHeader from '../styled/DrawerHeader'
import GeofenceTab from './geofence'
import RoutingTab from './routing'
import MenuItem from './MenuItem'
import { MySlide } from '../styled/Slide'
import ImportExport from './importExport'
import Settings from './settings'

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
        <>
          <DrawerHeader setStore={setStore}>Kōji</DrawerHeader>
          <Divider />
          <List>
            {TABS.map((text) => (
              <MenuItem key={text} name={text}>
                {{
                  Drawing: <GeofenceTab />,
                  Clustering: <RoutingTab />,
                  'Import / Export': <ImportExport />,
                  Settings: <Settings />,
                }[text] || null}
              </MenuItem>
            ))}
          </List>
        </>
      ) : (
        <Box
          sx={{
            width: '100%',
            height: '100vh',
            display: 'flex',
            alignItems: 'flex-start',
            justifyContent: 'center',
            transition: '0.50s ease',
          }}
        >
          <List>
            {TABS.map((text, i) => {
              const Icon = ICON_MAP[text] || null
              return (
                <Tooltip
                  key={text}
                  title={text}
                  enterDelay={0}
                  enterTouchDelay={10}
                  placement="right"
                  TransitionComponent={MySlide}
                >
                  <ListItemButton
                    onClick={() => {
                      setStore('drawer', true)
                      setStore('menuItem', text)
                    }}
                  >
                    {!!i && <Divider />} <Icon fontSize="large" />
                  </ListItemButton>
                </Tooltip>
              )
            })}
          </List>
        </Box>
      )}
    </Drawer>
  )
}
