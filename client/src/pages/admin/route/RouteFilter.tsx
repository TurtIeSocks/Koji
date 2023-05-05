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

import { useStatic } from '@hooks/useStatic'
import { RDM_ROUTES, UNOWN_ROUTES } from '@assets/constants'
import { BasicKojiEntry, KojiResponse } from '@assets/types'
import { fetchWrapper } from '@services/fetches'

export function RouteFilter() {
  const { scannerType } = useStatic.getState()
  const { data } = useQuery('unique_geofences', () =>
    fetchWrapper<KojiResponse<BasicKojiEntry[]>>(
      '/internal/admin/route/parent',
    ),
  )
  return (
    <Card sx={{ order: -1, width: 225 }}>
      <CardContent>
        {/* <SavedQueriesList /> */}
        <FilterLiveSearch />
        <FilterList label="Mode" icon={<AutoModeIcon />}>
          {[
            ...(scannerType === 'rdm' ? RDM_ROUTES : UNOWN_ROUTES),
            'unset',
          ].map((mode) => (
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
            {(data?.data || []).map((fence) => (
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
