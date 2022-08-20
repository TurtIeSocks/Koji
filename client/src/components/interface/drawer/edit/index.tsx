import React from 'react'
import { List, Divider } from '@mui/material'

import { useStore } from '@hooks/useStore'

import InstanceSelect from './Instance'

export default function SettingsTab() {
  const instance = useStore((s) => s.instance)
  const setSettings = useStore((s) => s.setSettings)

  return (
    <List dense>
      <InstanceSelect value={instance} setValue={setSettings} />
      <Divider sx={{ my: 2 }} />
    </List>
  )
}
