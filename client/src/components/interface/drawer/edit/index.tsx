import React from 'react'
import { List, Divider } from '@mui/material'

import { useStore } from '@hooks/useStore'

import InstanceSelect from './Instance'

export default function SettingsTab() {
  const instance = useStore((s) => s.instance)
  const setStore = useStore((s) => s.setStore)

  return (
    <List dense>
      <InstanceSelect value={instance} setValue={setStore} />
      <Divider sx={{ my: 2 }} />
    </List>
  )
}
