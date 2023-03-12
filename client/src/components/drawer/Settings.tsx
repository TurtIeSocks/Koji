import * as React from 'react'
import {
  Divider,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  MenuItem,
  Select,
} from '@mui/material'
import Logout from '@mui/icons-material/Logout'

import { KojiResponse, KojiTileServer } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { fetchWrapper } from '@services/fetches'

import Toggle from './inputs/Toggle'
import { MultiOptionList } from './inputs/MultiOptions'
import DateTime from './inputs/DateTime'
import ListSubheader from '../styled/Subheader'
import NumInput from './inputs/NumInput'

export default function Settings() {
  const tileServers = useStatic((s) => s.tileServers)
  const tileServer = usePersist((s) => s.tileServer)

  React.useEffect(() => {
    fetchWrapper<KojiResponse<KojiTileServer[]>>(
      '/internal/admin/tileserver/all/',
    ).then((data) => data && useStatic.setState({ tileServers: data.data }))
  }, [])

  return (
    <List>
      <ListSubheader disableGutters>Markers</ListSubheader>
      <MultiOptionList
        field="data"
        buttons={['all', 'area', 'bound']}
        label="Query Type"
        hideLabel
        type="select"
      />
      <DateTime field="last_seen" />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Other</ListSubheader>
      <Toggle field="loadingScreen" />
      <Toggle field="simplifyPolygons" />
      <Toggle field="showRouteIndex" />
      <ListItemButton
        onClick={() =>
          useStatic.setState((prev) => ({
            dialogs: {
              ...prev.dialogs,
              keyboard: true,
            },
          }))
        }
      >
        Keyboard Shortcuts
      </ListItemButton>
      {tileServers.length ? (
        <ListItem>
          <Select
            value={tileServer}
            fullWidth
            onChange={({ target }) => {
              usePersist.setState({ tileServer: target.value })
            }}
          >
            {tileServers.map(({ id, name, url }) => (
              <MenuItem key={id} value={url}>
                {name}
              </MenuItem>
            ))}
          </Select>
        </ListItem>
      ) : (
        <ListItemText
          primary="Set TileServer"
          secondary="Upload from the admin panel"
          sx={{ px: 2 }}
        />
      )}
      {process.env.NODE_ENV === 'development' && (
        <>
          <ListSubheader disableGutters>Dev Settings</ListSubheader>
          <Toggle field="nativeLeaflet" />
          <Toggle field="colorByGeohash" />
          <NumInput field="geohashPrecision" />
        </>
      )}
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
