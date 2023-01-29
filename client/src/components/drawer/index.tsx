import * as React from 'react'
import { Box, List, Divider, Tabs, Tab, Slide } from '@mui/material'
import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs'
import { LocalizationProvider } from '@mui/x-date-pickers/LocalizationProvider'
import type {} from '@mui/x-date-pickers/themeAugmentation'

import { ICON_MAP, TABS } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'
import { useShapes } from '@hooks/useShapes'

import { Drawer } from '../styled/Drawer'
import DrawerHeader from '../styled/DrawerHeader'
import DrawingTab from './Drawing'
import RoutingTab from './Routing'
import ImportExport from './manage'
import Settings from './Settings'
import MiniItem from './MiniItem'
import Layers from './Layers'
import GeojsonTab from './Geojson'

interface TabPanelProps {
  children?: React.ReactNode
  index: number
  value: number
}

function TabPanel({ children, value, index }: TabPanelProps) {
  return (
    <Box
      role="tabpanel"
      hidden={value !== index}
      display="block"
      maxHeight="100vh"
      overflow="auto"
    >
      {value === index && children}
    </Box>
  )
}

interface Props {
  drawerWidth: number
}

export default function DrawerIndex({ drawerWidth }: Props) {
  const drawer = usePersist((s) => s.drawer)
  const menuItem = usePersist((s) => s.menuItem)
  const { setStore } = usePersist.getState()

  const [tab, setTab] = React.useState(TABS.indexOf(menuItem) || 0)

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

  const handleChange = (_e: React.SyntheticEvent, newValue: number) => {
    setStore('menuItem', TABS[newValue])
    setTab(newValue)
  }

  React.useEffect(() => {
    window.addEventListener('keydown', handleKeyPress)
    return () => window.removeEventListener('keydown', handleKeyPress)
  }, [])

  React.useEffect(() => {
    setTab(TABS.indexOf(menuItem) || 0)
  }, [menuItem])

  return (
    <Drawer
      variant="permanent"
      open={drawer}
      drawerWidth={drawerWidth}
      onClose={toggleDrawer}
      onMouseEnter={() => useShapes.getState().setters.activeRoute()}
    >
      {drawer ? (
        <LocalizationProvider dateAdapter={AdapterDayjs}>
          <DrawerHeader>Kōji</DrawerHeader>
          <Divider />
          <Box
            sx={{
              flexGrow: 1,
              bgcolor: 'background.paper',
              display: 'flex',
              height: '100%',
            }}
          >
            <Tabs
              orientation="vertical"
              variant="scrollable"
              value={tab}
              onChange={handleChange}
              sx={{ borderRight: 1, borderColor: 'divider' }}
            >
              {TABS.map((text, i) => {
                const Icon = ICON_MAP[text]
                return (
                  <Tab
                    key={text}
                    value={i}
                    icon={<Icon />}
                    sx={{ p: 0, m: 0, minWidth: 45 }}
                  />
                )
              })}
            </Tabs>
            {TABS.map((text, i) => (
              <TabPanel key={text} value={tab} index={i}>
                {{
                  Drawing: <DrawingTab />,
                  Layers: <Layers />,
                  Clustering: <RoutingTab />,
                  Manage: <ImportExport />,
                  Settings: <Settings />,
                  Geojson: <GeojsonTab />,
                }[text] || null}
              </TabPanel>
            ))}
          </Box>
        </LocalizationProvider>
      ) : (
        <Slide in={!drawer} direction="left">
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
              <MiniItem text="Open" i={0} />
              {TABS.map((text, i) => (
                <MiniItem key={text} text={text} i={i} />
              ))}
            </List>
          </Box>
        </Slide>
      )}
    </Drawer>
  )
}
