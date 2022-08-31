import React from 'react'
import {
  Drawer,
  Box,
  List,
  Divider,
  ListItemButton,
  ListItemIcon,
  ListItem,
  Tabs,
  Tab,
} from '@mui/material'
import { ChevronRight, ContentCopy } from '@mui/icons-material'

import { TABS } from '@assets/constants'
import { useStore } from '@hooks/useStore'
import { useStatic } from '@hooks/useStatic'

import DrawerHeader from '../styled/DrawerHeader'
import ListSubheader from '../styled/Subheader'
import ExportRoute from '../dialogs/ExportRoute'
import GeofenceTab from './geofence'
import RoutingTab from './routing'
import BtnGroup from './inputs/BtnGroup'
import Toggle from './inputs/Toggle'
import PolygonDialog from '../dialogs/Polygon'

interface Props {
  drawerWidth: number
}

export default function DrawerIndex({ drawerWidth }: Props) {
  const setStore = useStore((s) => s.setStore)
  const tab = useStore((s) => s.tab)
  const drawer = useStore((s) => s.drawer)
  const pokestop = useStore((s) => s.pokestop)
  const gym = useStore((s) => s.gym)
  const spawnpoint = useStore((s) => s.spawnpoint)
  const data = useStore((s) => s.data)
  const nativeLeaflet = useStore((s) => s.nativeLeaflet)

  const geojson = useStatic((s) => s.geojson)

  const [open, setOpen] = React.useState('')

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
            <List key={t} hidden={tab !== i} dense>
              {{
                geofences: <GeofenceTab />,
                routing: <RoutingTab />,
              }[t] || null}
            </List>
          ))}
          <List dense>
            <Divider sx={{ my: 2 }} />
            <ListSubheader disableGutters>Markers</ListSubheader>
            <Toggle field="pokestop" value={pokestop} setValue={setStore} />
            <Toggle field="gym" value={gym} setValue={setStore} />
            <Toggle field="spawnpoint" value={spawnpoint} setValue={setStore} />
            <Toggle
              field="nativeLeaflet"
              value={nativeLeaflet}
              setValue={setStore}
            />
            <ListItem>
              <BtnGroup
                field="data"
                value={data}
                setValue={setStore}
                buttons={['all', 'bound', 'area']}
              />
            </ListItem>
            <Divider sx={{ my: 2 }} />
            <ListItemButton onClick={() => setOpen('polygon')}>
              <ListItemIcon>
                <ContentCopy />
              </ListItemIcon>
              Import Polygon
            </ListItemButton>
            <ListItemButton onClick={() => setOpen('route')}>
              <ListItemIcon>
                <ContentCopy />
              </ListItemIcon>
              Export Route
            </ListItemButton>
          </List>
          <PolygonDialog
            mode="import"
            open={open}
            setOpen={setOpen}
            feature={geojson}
          />
          <ExportRoute open={open} setOpen={setOpen} />
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
