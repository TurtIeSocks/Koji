import React from 'react'
import { Drawer, Box, List, Divider } from '@mui/material'
import { ChevronRight } from '@mui/icons-material'

import { useStore } from '@hooks/useStore'

import DrawerHeader from '../styled/DrawerHeader'
import InstanceSelect from './Instance'
import NumInput from './NumInput'
import BtnGroup from './BtnGroup'
import Toggle from './Toggle'

interface Props {
  drawer: boolean
  setDrawer: (drawer: boolean) => void
  drawerWidth: number
}

export default function DrawerIndex({ drawer, setDrawer, drawerWidth }: Props) {
  const instance = useStore((s) => s.instance)
  const radius = useStore((s) => s.radius)
  const mode = useStore((s) => s.mode)
  const category = useStore((s) => s.category)
  const generations = useStore((s) => s.generations)
  const setSettings = useStore((s) => s.setSettings)
  const pokestop = useStore((s) => s.pokestop)
  const gym = useStore((s) => s.gym)
  const spawnpoint = useStore((s) => s.spawnpoint)
  const data = useStore((s) => s.data)
  const showCircles = useStore((s) => s.showCircles)
  const showLines = useStore((s) => s.showLines)
  const showPolygon = useStore((s) => s.showPolygon)
  const renderer = useStore((s) => s.renderer)

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
            <InstanceSelect value={instance} setValue={setSettings} />
            <Divider sx={{ my: 2 }} />
            <NumInput field="radius" value={radius} setValue={setSettings} />
            <NumInput
              field="generations"
              value={generations}
              setValue={setSettings}
            />
            <Divider sx={{ my: 2 }} />
            <BtnGroup
              field="mode"
              value={mode}
              setValue={setSettings}
              buttons={['cluster', 'route', 'bootstrap']}
            />
            <BtnGroup
              field="category"
              value={category}
              setValue={setSettings}
              buttons={['pokestop', 'gym', 'spawnpoint']}
              disabled={mode === 'bootstrap'}
            />
            <Divider sx={{ my: 2 }} />
            <Toggle field="pokestop" value={pokestop} setValue={setSettings} />
            <Toggle field="gym" value={gym} setValue={setSettings} />
            <Toggle
              field="spawnpoint"
              value={spawnpoint}
              setValue={setSettings}
            />
            <BtnGroup
              field="data"
              value={data}
              setValue={setSettings}
              buttons={['all', 'bound', 'area']}
            />
            <Divider sx={{ my: 2 }} />
            <Toggle
              field="showCircles"
              value={showCircles}
              setValue={setSettings}
            />
            <Toggle
              field="showLines"
              value={showLines}
              setValue={setSettings}
            />
            <Toggle
              field="showPolygon"
              value={showPolygon}
              setValue={setSettings}
            />
            <BtnGroup
              field="renderer"
              value={renderer}
              setValue={setSettings}
              buttons={['quality', 'performance']}
            />
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
