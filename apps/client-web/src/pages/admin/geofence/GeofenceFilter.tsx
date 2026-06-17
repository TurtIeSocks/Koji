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

import { UNOWN_FENCES } from '@assets/constants'
import { FeatureCollection, KojiProject } from '@assets/types'
import { fetchWrapper } from '@services/fetches'

export function GeofenceFilter() {
  const projectData = useGetList<KojiProject>('project', {
    sort: { field: 'name', order: 'ASC' },
  })
  // v2 has no dedicated "parent geofences" endpoint — list all geofences
  // (GeoJSON) and derive {id, name} from each feature's properties.
  // TODO(v2-gap): the v1 `/internal/admin/geofence/parent` (only fences USED as a
  // parent) is gone; this now lists every geofence as a candidate parent.
  const { data } = useQuery('parents', () =>
    fetchWrapper<FeatureCollection>('/api/v2/geofences').then((fc) =>
      (fc?.features || []).map((f) => ({
        id: (f.properties?.id ?? f.properties?.__id ?? f.id) as number,
        name: (f.properties?.name ?? f.properties?.__name ?? '') as string,
      })),
    ),
  )
  return (
    <Card sx={{ order: -1, width: 200 }}>
      <CardContent>
        {/* <SavedQueriesList /> */}
        <FilterLiveSearch />
        <FilterList label="Project" icon={<AccountTree />}>
          <FilterListItem key="unset" label="unset" value={{ project: 0 }} />
          {(projectData?.data || []).map((project) => (
            <FilterListItem
              key={project.id}
              label={project.name}
              value={{ project: project.id }}
            />
          ))}
        </FilterList>
        <FilterList label="Parent" icon={<SupervisedUserCircleIcon />}>
          <FilterListItem key="unset" label="unset" value={{ parent: 0 }} />
          {(data || []).map((parent) => (
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
          {['unset', ...UNOWN_FENCES].map((mode) => (
            <FilterListItem key={mode} label={mode} value={{ mode }} />
          ))}
        </FilterList>
      </CardContent>
    </Card>
  )
}
