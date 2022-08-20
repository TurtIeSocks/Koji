import React from 'react'
import {
  Drawer,
  Box,
  List,
  Divider,
  ListItemButton,
  ListItemIcon,
  Tabs,
  Tab,
} from '@mui/material'
import { ChevronRight, ContentCopy } from '@mui/icons-material'

import { TABS } from '@assets/constants'
import { useStore } from '@hooks/useStore'

import DrawerHeader from '../styled/DrawerHeader'
import Export from './Export'
import EditTab from './edit'
import SettingsTab from './settings'
import CreateTab from './create'

interface Props {
  drawerWidth: number
}

export default function DrawerIndex({ drawerWidth }: Props) {
  const setStore = useStore((s) => s.setStore)
  const tab = useStore((s) => s.tab)
  const drawer = useStore((s) => s.drawer)

  const [open, setOpen] = React.useState(false)

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
          <DrawerHeader setStore={setStore}>Kōji</DrawerHeader>
          <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
            <Tabs
              value={tab}
              onChange={(_e, newValue) => setStore('tab', newValue)}
            >
              {TABS.map((t) => (
                <Tab
                  key={t}
                  label={t}
                  sx={{ width: `calc(100% / ${TABS.length})` }}
                />
              ))}
            </Tabs>
          </Box>
          {TABS.map((t, i) => (
            <Box key={t} hidden={tab !== i}>
              {{
                create: <CreateTab />,
                edit: <EditTab />,
                settings: <SettingsTab />,
              }[t] || null}
            </Box>
          ))}
          {tab !== 2 && (
            <List dense>
              <Divider sx={{ my: 2 }} />
              <ListItemButton onClick={() => setOpen(true)}>
                <ListItemIcon>
                  <ContentCopy />
                </ListItemIcon>
                Export
              </ListItemButton>
            </List>
          )}
          <Export open={open} setOpen={setOpen} />
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
          onClick={() => setStore('drawer', true)}
        >
          <ChevronRight fontSize="small" />
        </Box>
      )}
    </Drawer>
  )
}
