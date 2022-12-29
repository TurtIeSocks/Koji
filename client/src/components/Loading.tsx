/* eslint-disable no-nested-ternary */
import * as React from 'react'
import shallow from 'zustand/shallow'
import Backdrop from '@mui/material/Backdrop'
import CircularProgress from '@mui/material/CircularProgress'
import Divider from '@mui/material/Divider'
import Typography from '@mui/material/Typography'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

import { usePersist } from '@hooks/usePersist'
import { useStatic } from '@hooks/useStatic'
import { fromSnakeCase } from '@services/utils'

export default function Loading() {
  const loading = useStatic((s) => s.loading)
  const totalLoadingTime = useStatic((s) => s.totalLoadingTime)
  const setStatic = useStatic((s) => s.setStatic)

  const settings = usePersist(
    (s) => ({
      mode: s.mode,
      radius: s.radius,
      category: s.category,
      min_points: s.min_points,
      fast: s.fast,
      routing_time: s.routing_time,
    }),
    shallow,
  )
  const [time, setTime] = React.useState(0)

  const loadingStarted = Object.keys(loading).length

  React.useEffect(() => {
    let interval: NodeJS.Timeout
    if (!totalLoadingTime) {
      interval = setInterval(() => {
        setTime((t) => t + 1)
      }, 1000)
    }
    return () => {
      if (interval) {
        clearInterval(interval)
      }
      setTime(0)
    }
  }, [loadingStarted])

  return loadingStarted ? (
    <Grid2
      container
      component={Backdrop}
      open={!!Object.keys(loading).length}
      onClick={() => setStatic('loading', {})}
      sx={{
        color: '#fff',
        zIndex: (theme) => theme.zIndex.drawer + 1,
        p: 4,
        overflow: 'auto',
        height: '100vh',
        bgcolor: 'rgba(0, 0, 0, 0.8)',
      }}
    >
      <Grid2 xs={totalLoadingTime ? 6 : 12}>
        <Typography variant="h2" color="secondary">
          {totalLoadingTime
            ? 'Stats'
            : `Loading...${(
                (Object.values(loading).filter(Boolean).length /
                  Object.keys(loading).length) *
                100
              ).toFixed(2)}
          % - ${time}s`}
        </Typography>
      </Grid2>
      <Grid2
        xs={6}
        container
        sx={{ display: totalLoadingTime ? 'flex' : 'none' }}
      >
        {Object.entries(settings).map(([key, value]) => (
          <Grid2 key={key} xs={4} sm={2}>
            <Typography color="primary">{fromSnakeCase(key)}</Typography>
            <Typography variant="subtitle2">
              {value.toString()}
              {key === 'routing_time' ? 's' : key === 'radius' ? 'm' : ''}
            </Typography>
          </Grid2>
        ))}
      </Grid2>
      <Divider
        flexItem
        sx={{ width: '95%', mx: 'auto', height: 8, mt: 2, color: 'white' }}
      />
      {Object.entries(loading).map(([key, value]) => (
        <Grid2 key={key} container xs={12} md={6}>
          <Grid2 xs={12} width="100%" my={2}>
            <Typography variant="h4" color="primary">
              {key}
            </Typography>
            {value && (
              <Typography variant="caption">
                Time: {value.cluster_time.toFixed(3)}s
              </Typography>
            )}
          </Grid2>

          {value ? (
            <Grid2 xs={12} container>
              <Grid2 xs={6} sm={4} container>
                <Grid2 xs={12}>
                  <Typography color="secondary" variant="h5">
                    Clustering
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Total: {value.total_clusters.toLocaleString()}
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Best Coverage:{' '}
                    {value.best_cluster_point_count.toLocaleString()}
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Best Count: {value.best_clusters.length.toLocaleString()}
                  </Typography>
                </Grid2>
              </Grid2>

              <Grid2 xs={6} sm={4} container>
                <Grid2 xs={12}>
                  <Typography color="secondary" variant="h5">
                    Points
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Total: {value.total_points.toLocaleString()}
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Covered: {value.points_covered.toLocaleString()}
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Average:{' '}
                    {(
                      value.points_covered / (value.total_clusters || 1)
                    ).toLocaleString()}
                  </Typography>
                </Grid2>
              </Grid2>

              <Grid2 xs={12} sm={4} container>
                <Grid2 xs={12}>
                  <Typography color="secondary" variant="h5">
                    Distance
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Total: {value.total_distance.toFixed(3)}m
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Longest: {value.longest_distance.toFixed(3)}m
                  </Typography>
                </Grid2>
                <Grid2 xs={12}>
                  <Typography variant="subtitle2">
                    Average:{' '}
                    {(
                      value.total_distance / (value.total_clusters || 1)
                    ).toFixed(3)}
                    m
                  </Typography>
                </Grid2>
              </Grid2>
            </Grid2>
          ) : (
            <Grid2 xs={6}>
              <CircularProgress
                size={`calc(100vh / ${Object.keys(loading).length * 3})`}
              />
            </Grid2>
          )}
          <Divider
            flexItem
            sx={{ width: '90%', mx: 'auto', height: 8, mt: 2, color: 'white' }}
          />
        </Grid2>
      ))}
      {!!totalLoadingTime && (
        <Grid2 xs={12} container pt={3}>
          <Grid2 xs={4}>
            <Typography variant="h4">Total Time:</Typography>
          </Grid2>
          <Grid2 xs={4}>
            <Typography variant="h4">{totalLoadingTime / 1000}s</Typography>
          </Grid2>
          <Grid2 xs={4}>
            <Typography variant="caption">Click anywhere to close</Typography>
          </Grid2>
        </Grid2>
      )}
    </Grid2>
  ) : null
}
