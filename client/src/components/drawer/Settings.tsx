import * as React from 'react'
import {
  Divider,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from '@mui/material'
import Logout from '@mui/icons-material/Logout'

import Toggle from './inputs/Toggle'
import { MultiOptionList } from './inputs/MultiOptions'
import DateTime from './inputs/DateTime'
import ListSubheader from '../styled/Subheader'

export default function Settings() {
  return (
    <List sx={{ width: 275 }}>
      <ListSubheader disableGutters>Markers</ListSubheader>
      <MultiOptionList
        field="data"
        buttons={['all', 'area', 'bound']}
        label="Query Type"
        hideLabel
        type="select"
      />
      <DateTime field="last_seen" />
      {process.env.NODE_ENV === 'development' && (
        <Toggle field="nativeLeaflet" />
      )}
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Other</ListSubheader>
      <Toggle field="loadingScreen" />
      <Toggle field="simplifyPolygons" />
      <Toggle field="showRouteIndex" />
      <ListItemButton href="/config/logout">
        <ListItemIcon>
          <Logout color="secondary" />
        </ListItemIcon>
        <ListItemText
          primary="Logout"
          primaryTypographyProps={{ color: 'secondary' }}
        />
      </ListItemButton>
    </List>
  )
}
