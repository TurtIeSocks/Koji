/* eslint-disable react/no-array-index-key */
import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import Typography from '@mui/material/Typography'

import { AdminProject, KojiResponse, FeatureCollection } from '@assets/types'
import { Checkbox, Divider, MenuItem, Select } from '@mui/material'
import ReactWindow from '@components/ReactWindow'
import { useStatic } from '@hooks/useStatic'
import {
  RDM_FENCES,
  RDM_ROUTES,
  UNOWN_FENCES,
  UNOWN_ROUTES,
} from '@assets/constants'
import ProjectsAc from '@components/drawer/inputs/ProjectsAC'
import { getData } from '@services/fetches'
import { useDbCache } from '@hooks/useDbCache'

const AssignStep = React.forwardRef<
  HTMLDivElement,
  {
    handleChange: (geojson: FeatureCollection) => void
    geojson: FeatureCollection
    refGeojson: FeatureCollection
    routeMode?: boolean
  }
>(({ handleChange, geojson, refGeojson, routeMode = false }, ref) => {
  const {
    allProjects,
    allGeofences,
    allFenceMode,
    allRouteMode,
    checked,
    nameProp,
  } = useStatic((s) => s.importWizard)
  const scannerType = useStatic((s) => s.scannerType)

  const innerRef = React.useRef<HTMLDivElement>(null)

  React.useEffect(() => {
    getData<KojiResponse<Omit<AdminProject, 'related'>[]>>(
      '/internal/admin/project/all/',
    ).then((res) => {
      if (res) {
        useStatic.setState({
          projects: Object.fromEntries(
            res.data.map((project) => [
              project.id,
              {
                ...project,
                related: [],
              },
            ]),
          ),
        })
      }
    })
  }, [])

  React.useEffect(() => {
    useStatic.setState((prev) => ({
      importWizard: {
        ...prev.importWizard,
        checked: Object.fromEntries(
          geojson.features.map((feature) => [
            feature.id,
            checked[feature.id || ''] ?? true,
          ]),
        ),
      },
    }))
  }, [])

  const all = Object.values(checked).every((val) => val)
  const some = !all && Object.values(checked).some((val) => val)

  const sorted = React.useMemo(
    () =>
      refGeojson.features
        .filter((feat) =>
          feat.geometry.type.includes(routeMode ? 'Point' : 'Polygon'),
        )
        .sort((a, b) => {
          const aName = a.properties?.[nameProp]
          const bName = b.properties?.[nameProp]
          return typeof aName === 'string' && typeof bName === 'string'
            ? aName.localeCompare(bName)
            : 0
        }),
    [nameProp, routeMode],
  )

  return (
    <Grid2 container ref={ref} sx={{ width: '100%' }}>
      <Grid2 xs={1} mt={1} />
      <Grid2 xs={3} mt={1}>
        <Typography variant="h6" align="center">
          Feature
        </Typography>
      </Grid2>
      <Grid2 xs={3} mt={1}>
        <Typography variant="h6" align="center">
          Mode
        </Typography>
      </Grid2>
      <Grid2 xs={5} mt={1}>
        <Typography variant="h6" align="center">
          {routeMode ? 'Geofence Parent' : 'Projects'}
        </Typography>
      </Grid2>
      <Grid2 xs={1} mt={1}>
        <Checkbox
          checked={all}
          indeterminate={some}
          onClick={() =>
            useStatic.setState((prev) => ({
              importWizard: {
                ...prev.importWizard,
                checked: Object.fromEntries(
                  Object.keys(checked).map((k) => [k, !all && !some]),
                ),
              },
            }))
          }
        />
      </Grid2>
      <Grid2 xs={3} mt={1}>
        <Typography variant="subtitle2" align="center">
          All
        </Typography>
      </Grid2>
      <Grid2 xs={3} mt={1}>
        <Select
          value={routeMode ? allRouteMode : allFenceMode}
          size="small"
          sx={{ width: '80%' }}
          onChange={(e) => {
            useStatic.setState((prev) => ({
              importWizard: {
                ...prev.importWizard,
                [routeMode ? 'allRouteMode' : 'allFenceMode']: e.target.value,
              },
            }))
            handleChange({
              ...geojson,
              features: geojson.features.map((feature) => ({
                ...feature,
                properties: {
                  ...feature.properties,
                  mode: feature.geometry.type.includes(
                    routeMode ? 'Polygon' : 'Point',
                  )
                    ? feature.properties.mode
                    : e.target.value,
                },
              })),
            })
          }}
        >
          <MenuItem value="" />
          {(scannerType === 'rdm'
            ? routeMode
              ? RDM_ROUTES
              : RDM_FENCES
            : routeMode
            ? UNOWN_ROUTES
            : UNOWN_FENCES
          ).map((mode) => (
            <MenuItem key={mode} value={mode}>
              {mode}
            </MenuItem>
          ))}
        </Select>
      </Grid2>
      <Grid2 xs={5} mt={1}>
        {routeMode ? (
          <Select
            size="small"
            sx={{ width: '80%', mx: 'auto' }}
            value={allGeofences}
            onChange={({ target }) => {
              useStatic.setState((prev) => ({
                importWizard: {
                  ...prev.importWizard,
                  allGeofences: +target.value ? +target.value : target.value,
                },
              }))
              handleChange({
                ...geojson,
                features: geojson.features.map((feature) => ({
                  ...feature,
                  properties: {
                    ...feature.properties,
                    geofence_id: feature.geometry.type.includes('Polygon')
                      ? feature.properties.geofence_id
                      : +target.value
                      ? +target.value
                      : target.value,
                  },
                })),
              })
            }}
          >
            {Object.values(useDbCache.getState().geofence).map((t) => (
              <MenuItem key={t.id} value={t.id}>
                {t.name}
              </MenuItem>
            ))}
            {refGeojson.features
              .filter((feat) => feat.geometry.type.includes('Polygon'))
              .sort((a, b) => {
                const aName = a.properties?.[nameProp]
                const bName = b.properties?.[nameProp]
                return typeof aName === 'string' && typeof bName === 'string'
                  ? aName.localeCompare(bName)
                  : 0
              })
              .map((t) => (
                <MenuItem key={t.id} value={t.properties?.[nameProp]}>
                  {t.properties?.[nameProp]}
                </MenuItem>
              ))}
          </Select>
        ) : (
          <ProjectsAc
            value={allProjects}
            setValue={(newValue) => {
              handleChange({
                ...geojson,
                features: geojson.features.map((feature) => ({
                  ...feature,
                  properties: {
                    ...feature.properties,
                    projects: feature.geometry.type.includes('Polygon')
                      ? newValue
                      : undefined,
                  },
                })),
              })
              useStatic.setState((prev) => ({
                importWizard: {
                  ...prev.importWizard,
                  allProjects: newValue,
                },
              }))
            }}
          />
        )}
      </Grid2>
      <Divider sx={{ width: '100%', my: 1 }} />
      <Grid2 xs={12} ref={innerRef}>
        <div key={sorted.length}>
          <ReactWindow
            rows={sorted}
            itemSize={60}
            data={{ geojson }}
            height={innerRef?.current?.clientHeight || 350}
          >
            {({ style, data, index }) => {
              const refFeature = data.rows[index]
              const feature = data.geojson.features.find(
                (feat) => feat.id === refFeature.id,
              )
              if (!feature) return null
              const isActive = feature && checked[feature.id || '']

              return (
                <Grid2
                  key={`${feature?.properties?.name}`}
                  container
                  style={style}
                >
                  <Grid2 xs={1}>
                    <Checkbox
                      checked={isActive}
                      onChange={() => {
                        useStatic.setState((prev) => ({
                          importWizard: {
                            ...prev.importWizard,
                            checked: {
                              ...prev.importWizard.checked,
                              [feature.id as string]: !isActive,
                            },
                          },
                        }))
                      }}
                      color={isActive ? 'primary' : 'secondary'}
                    />
                  </Grid2>
                  <Grid2 xs={3}>
                    <Typography variant="subtitle2">
                      {feature.properties?.name || `Feature_${index}`}
                    </Typography>
                    <Typography variant="caption">
                      {feature.geometry.type}
                    </Typography>
                  </Grid2>
                  <Grid2 xs={3}>
                    <Select
                      size="small"
                      sx={{ width: '80%' }}
                      value={feature.properties?.mode || ''}
                      onChange={(e) => {
                        const newFeature = {
                          ...feature,
                          properties: {
                            ...feature.properties,
                            mode: e.target.value
                              ? (e.target.value as string)
                              : undefined,
                          },
                        }
                        handleChange({
                          ...geojson,
                          features: [
                            ...geojson.features.filter(
                              (f) => f.id !== feature.id,
                            ),
                            newFeature,
                          ],
                        })
                      }}
                    >
                      <MenuItem value="" />
                      {(scannerType === 'rdm'
                        ? routeMode
                          ? RDM_ROUTES
                          : RDM_FENCES
                        : routeMode
                        ? UNOWN_ROUTES
                        : UNOWN_FENCES
                      ).map((instanceType) => (
                        <MenuItem key={instanceType} value={instanceType}>
                          {instanceType}
                        </MenuItem>
                      ))}
                    </Select>
                  </Grid2>
                  <Grid2 xs={5}>
                    {routeMode ? (
                      <Select
                        size="small"
                        sx={{ width: '80%', mx: 'auto' }}
                        value={
                          feature.properties.geofence_id ||
                          data.geojson.features.find(
                            (f) =>
                              f.geometry.type.includes('Polygon') &&
                              f.properties[nameProp] ===
                                feature.properties[nameProp],
                          )?.properties?.[nameProp] ||
                          ''
                        }
                        onChange={({ target }) => {
                          const newFeature = {
                            ...feature,
                            properties: {
                              ...feature.properties,
                              geofence_id: +target.value
                                ? +target.value
                                : target.value,
                            },
                          }
                          handleChange({
                            ...geojson,
                            features: [
                              ...geojson.features.filter(
                                (f) => f.id !== feature.id,
                              ),
                              newFeature,
                            ],
                          })
                        }}
                      >
                        {Object.values(useDbCache.getState().geofence).map(
                          (t) => (
                            <MenuItem key={t.id} value={t.id}>
                              {t.name}
                            </MenuItem>
                          ),
                        )}
                        {refGeojson.features
                          .filter((feat) =>
                            feat.geometry.type.includes('Polygon'),
                          )
                          .sort((a, b) => {
                            const aName = a.properties?.[nameProp]
                            const bName = b.properties?.[nameProp]
                            return typeof aName === 'string' &&
                              typeof bName === 'string'
                              ? aName.localeCompare(bName)
                              : 0
                          })
                          .map((t) => (
                            <MenuItem
                              key={t.id}
                              value={t.properties?.[nameProp]}
                            >
                              {t.properties?.[nameProp]}
                            </MenuItem>
                          ))}
                      </Select>
                    ) : (
                      <ProjectsAc
                        value={feature.properties?.projects || []}
                        setValue={(newValue) => {
                          const newFeature = {
                            ...feature,
                            properties: {
                              ...feature.properties,
                              projects: newValue,
                            },
                          }
                          handleChange({
                            ...geojson,
                            features: [
                              ...geojson.features.filter(
                                (f) => f.id !== feature.id,
                              ),
                              newFeature,
                            ],
                          })
                        }}
                      />
                    )}
                  </Grid2>
                </Grid2>
              )
            }}
          </ReactWindow>
        </div>
      </Grid2>
    </Grid2>
  )
})

AssignStep.displayName = 'AssignStep'

export default AssignStep
