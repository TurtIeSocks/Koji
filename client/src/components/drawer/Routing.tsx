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

import {
  BOOTSTRAP_LEVELS,
  CALC_MODE,
  CATEGORIES,
  CLUSTERING_MODES,
  MODES,
  S2_CELL_LEVELS,
  SORT_BY,
  TTH,
} from '@assets/constants'
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
  const cluster_mode = usePersist((s) => s.cluster_mode)
  const calculation_mode = usePersist((s) => s.calculation_mode)
  const sort_by = usePersist((s) => s.sort_by)

  const scannerType = useStatic((s) => s.scannerType)
  const updateButton = useStatic((s) => s.updateButton)
  const isEditing = useStatic((s) =>
    Object.values(s.layerEditing).some((v) => v),
  )
  const routePlugins = useStatic((s) => s.route_plugins)

  const sortByOptions = React.useMemo(() => {
    return [...SORT_BY, ...routePlugins]
  }, [routePlugins])

  const fastest = cluster_mode === 'Fastest'
  return (
    <List dense sx={{ height: '90vh' }}>
      <ListSubheader>Calculation Modes</ListSubheader>
      <MultiOptionList field="mode" buttons={MODES} type="select" />
      <Collapse in={mode !== 'bootstrap'}>
        <MultiOptionList
          field="category"
          buttons={CATEGORIES}
          disabled={mode === 'bootstrap'}
          type="select"
        />
        <Collapse in={category === 'spawnpoint'}>
          <MultiOptionList field="tth" buttons={TTH} type="select" />
        </Collapse>
      </Collapse>
      <MultiOptionList
        field="calculation_mode"
        buttons={CALC_MODE}
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
        <MultiOptionList
          field="cluster_mode"
          hideLabel
          buttons={CLUSTERING_MODES}
          type="select"
        />
        <Collapse in={!fastest}>
          <NumInput field="cluster_split_level" min={1} max={20} />
        </Collapse>
        <Collapse in={!fastest}>
          <NumInput field="max_clusters" min={0} />
        </Collapse>
      </Collapse>

      <Divider sx={{ my: 2 }} />
      <ListSubheader>Routing</ListSubheader>
      <MultiOptionList field="sort_by" buttons={sortByOptions} type="select" />
      <Collapse in={!SORT_BY.some((sort) => sort === sort_by)}>
        <NumInput field="route_split_level" min={1} max={12} />
      </Collapse>

      <Divider sx={{ my: 2 }} />
      <ListSubheader>Saving</ListSubheader>
      <Toggle field="save_to_db" label="Save to Kōji Db" />
      <Toggle
        field="save_to_scanner"
        label="Save to Scanner Db"
        disabled={scannerType !== 'unown'}
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
