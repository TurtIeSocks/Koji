/* eslint-disable import/no-extraneous-dependencies */
import * as React from 'react'
import {
  // SavedQueriesList,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  useGetList,
} from 'react-admin'
import { useQuery } from 'react-query'
import { Card, CardContent } from '@mui/material'
import AutoModeIcon from '@mui/icons-material/AutoMode'
import AccountTree from '@mui/icons-material/AccountTree'
import MapIcon from '@mui/icons-material/Map'
import SupervisedUserCircleIcon from '@mui/icons-material/SupervisedUserCircle'

import { useStatic } from '@hooks/useStatic'
import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'
import { KojiGeofence, KojiProject, KojiResponse } from '@assets/types'
import { fetchWrapper } from '@services/fetches'

export function GeofenceFilter() {
  const { scannerType } = useStatic.getState()
  const projectData = useGetList<KojiProject>('project', {
    sort: { field: 'name', order: 'ASC' },
  })
  const { data } = useQuery('parents', () =>
    fetchWrapper<KojiResponse<KojiGeofence[]>>(
      '/internal/admin/geofence/parent',
    ),
  )
  return (
    <Card sx={{ order: -1, width: 200 }}>
      <CardContent>
        {/* <SavedQueriesList /> */}
        <FilterLiveSearch />
        <FilterList label="Project" icon={<AccountTree />}>
          {(projectData?.data || []).map((project) => (
            <FilterListItem
              key={project.id}
              label={project.name}
              value={{ project: project.id }}
            />
          ))}
        </FilterList>
        <FilterList label="Parent" icon={<SupervisedUserCircleIcon />}>
          {(data?.data || []).map((parent) => (
            <FilterListItem
              key={parent.id}
              label={parent.name}
              value={{ parent: parent.id }}
            />
          ))}
        </FilterList>
        <FilterList label="Geography Type" icon={<MapIcon />}>
          {['Polygon', 'MultiPolygon'].map((geotype) => (
            <FilterListItem key={geotype} label={geotype} value={{ geotype }} />
          ))}
        </FilterList>
        <FilterList label="Mode" icon={<AutoModeIcon />}>
          {[
            ...(scannerType === 'rdm' ? RDM_FENCES : UNOWN_FENCES),
            'unset',
          ].map((mode) => (
            <FilterListItem key={mode} label={mode} value={{ mode }} />
          ))}
        </FilterList>
      </CardContent>
    </Card>
  )
}
