import React from 'react'
import {
  Collapse,
  Divider,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
} from '@mui/material'
import Update from '@mui/icons-material/Update'

import { BOOTSTRAP_LEVELS, S2_CELL_LEVELS } from '@assets/constants'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { clusteringRouting } from '@services/fetches'

import ListSubheader from '../styled/Subheader'
import NumInput from './inputs/NumInput'
import { MultiOptionList } from './inputs/MultiOptions'
import Toggle from './inputs/Toggle'

export default function RoutingTab() {
  const mode = usePersist((s) => s.mode)
  const category = usePersist((s) => s.category)
  const fast = usePersist((s) => s.fast)
  const calculation_mode = usePersist((s) => s.calculation_mode)

  const [updateButton, scannerType, isEditing] = useStatic((s) => [
    s.updateButton,
    s.scannerType,
    Object.values(s.layerEditing).some((v) => v),
  ])

  React.useEffect(() => {
    if (category === 'pokestop' && scannerType === 'rdm') {
      usePersist.setState({ save_to_db: false, save_to_scanner: false })
    }
  }, [category])

  return (
    <List dense sx={{ height: '90vh' }}>
      <ListSubheader>Calculation Modes</ListSubheader>
      <MultiOptionList
        field="mode"
        buttons={['cluster', 'route', 'bootstrap']}
        type="select"
      />
      <Collapse in={mode !== 'bootstrap'}>
        <MultiOptionList
          field="category"
          buttons={['pokestop', 'gym', 'fort', 'spawnpoint']}
          disabled={mode === 'bootstrap'}
          type="select"
        />
        <Collapse in={category === 'spawnpoint'}>
          <MultiOptionList
            field="tth"
            buttons={['All', 'Known', 'Unknown']}
            type="select"
          />
        </Collapse>
      </Collapse>
      <MultiOptionList
        field="calculation_mode"
        buttons={['Radius', 'S2']}
        label="Strategy"
        hideLabel
        type="select"
      />
      <Collapse in={calculation_mode === 'Radius'}>
        <NumInput field="radius" />
      </Collapse>
      <Collapse in={calculation_mode === 'S2'}>
        <MultiOptionList
          field="s2_level"
          label="S2 Level"
          hideLabel
          buttons={S2_CELL_LEVELS}
          type="select"
          itemLabel={(v) => `Level ${v}`}
        />
        <MultiOptionList
          field="s2_size"
          label="S2 Size"
          hideLabel
          buttons={BOOTSTRAP_LEVELS}
          type="select"
          itemLabel={(v) => `${v}x${v}`}
        />
      </Collapse>

      <Collapse in={mode !== 'bootstrap' && calculation_mode === 'Radius'}>
        <Divider sx={{ my: 2 }} />
        <ListSubheader>Clustering</ListSubheader>
        <NumInput field="min_points" />
        <Toggle field="fast" />
        <Collapse in={!fast}>
          <Toggle field="only_unique" />
        </Collapse>
      </Collapse>
      <Collapse in={mode === 'cluster' && !fast}>
        <MultiOptionList
          field="sort_by"
          buttons={['GeoHash', 'ClusterCount', 'Random']}
          disabled={mode !== 'cluster'}
          type="select"
        />
      </Collapse>

      <Collapse in={mode === 'route'}>
        <Divider sx={{ my: 2 }} />
        <ListSubheader>Routing</ListSubheader>
        <NumInput
          field="route_split_level"
          disabled={mode !== 'route'}
          min={1}
          max={12}
        />
      </Collapse>

      <Divider sx={{ my: 2 }} />
      <ListSubheader>Saving</ListSubheader>
      <Toggle
        field="save_to_db"
        label="Save to Kōji Db"
        disabled={
          scannerType === 'rdm' &&
          category === 'pokestop' &&
          mode !== 'bootstrap'
        }
      />
      <Toggle
        field="save_to_scanner"
        label="Save to Scanner Db"
        disabled={
          !useStatic.getState().dangerous ||
          (scannerType === 'rdm' &&
            category === 'pokestop' &&
            mode !== 'bootstrap')
        }
      />
      <Toggle field="skipRendering" />
      <ListItemButton
        color="primary"
        disabled={isEditing || !!updateButton}
        onClick={async () => {
          useStatic.setState({ updateButton: true })
          await clusteringRouting().then(() => {
            useStatic.setState({ updateButton: false })
          })
        }}
      >
        <ListItemIcon>
          <Update color="secondary" />
        </ListItemIcon>
        <ListItemText
          primary="Update"
          primaryTypographyProps={{ color: 'secondary' }}
        />
      </ListItemButton>
    </List>
  )
}
