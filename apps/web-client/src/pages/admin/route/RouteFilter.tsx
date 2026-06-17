/* eslint-disable import/no-extraneous-dependencies */
import * as React from 'react'
import {
  // SavedQueriesList,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
} from 'react-admin'
import { useQuery } from 'react-query'
import { Card, CardContent } from '@mui/material'
import AutoModeIcon from '@mui/icons-material/AutoMode'
import SupervisedUserCircleIcon from '@mui/icons-material/SupervisedUserCircle'

import { UNOWN_ROUTES } from '@assets/constants'
import { FeatureCollection } from '@assets/types'
import { fetchWrapper } from '@services/fetches'

export function RouteFilter() {
  // v2 has no "geofences that own a route" endpoint — list all geofences
  // (GeoJSON) and offer them as the geofence filter.
  // TODO(v2-gap): v1 `/internal/admin/route/parent` (only geofences WITH routes)
  // is gone; this lists every geofence as a route-geofence filter candidate.
  const { data } = useQuery('unique_geofences', () =>
    fetchWrapper<FeatureCollection>('/api/v2/geofences').then((fc) =>
      (fc?.features || []).map((f) => ({
        id: (f.properties?.id ?? f.properties?.__id ?? f.id) as number,
        name: (f.properties?.name ?? f.properties?.__name ?? '') as string,
      })),
    ),
  )
  return (
    <Card sx={{ order: -1, width: 225 }}>
      <CardContent>
        {/* <SavedQueriesList /> */}
        <FilterLiveSearch />
        <FilterList label="Mode" icon={<AutoModeIcon />}>
          {[...UNOWN_ROUTES, 'unset'].map((mode) => (
            <FilterListItem key={mode} label={mode} value={{ mode }} />
          ))}
        </FilterList>
        <FilterList label="Points" icon={<AutoModeIcon />}>
          {[0, 1, 5, 10, 25].map((count, i, arr) => (
            <FilterListItem
              key={count}
              label={`${(count * 1000).toLocaleString()} ${
                i === arr.length - 1
                  ? '<'
                  : `- ${(arr[i + 1] * 1000).toLocaleString()}`
              }`}
              value={{
                pointsmin: count * 1000,
                pointsmax: i === arr.length - 1 ? undefined : arr[i + 1] * 1000,
              }}
            />
          ))}
        </FilterList>
        <FilterList label="Geofence" icon={<SupervisedUserCircleIcon />}>
          <div style={{ maxHeight: 400, overflow: 'auto' }}>
            {(data || []).map((fence) => (
              <FilterListItem
                key={fence.id}
                label={fence.name}
                value={{ geofenceid: fence.id }}
              />
            ))}
          </div>
        </FilterList>
      </CardContent>
    </Card>
  )
}
