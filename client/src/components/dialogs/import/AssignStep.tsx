/* eslint-disable react/no-array-index-key */
import * as React from 'react'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import Typography from '@mui/material/Typography'
import type { FeatureCollection } from 'geojson'

import { ClientProject } from '@assets/types'
import { Checkbox, Divider, MenuItem, Select } from '@mui/material'
import ReactWindow from '@components/ReactWindow'
import { useStatic } from '@hooks/useStatic'
import { RDM_FENCES, UNOWN_FENCES } from '@assets/constants'
import ProjectsAc from '@components/drawer/inputs/ProjectsAC'

const AssignStep = React.forwardRef<
  HTMLDivElement,
  {
    handleChange: (geojson: FeatureCollection) => void
    geojson: FeatureCollection
    refGeojson: FeatureCollection
  }
>(({ handleChange, geojson, refGeojson }, ref) => {
  const { allProjects, allType, checked, nameProp } = useStatic(
    (s) => s.importWizard,
  )
  const scannerType = useStatic((s) => s.scannerType)

  const innerRef = React.useRef<HTMLDivElement>(null)

  React.useEffect(() => {
    fetch('/internal/admin/project/all/')
      .then((res) => res.json())
      .then((data) => {
        return useStatic.setState({
          projects: Object.fromEntries(
            data.data.map((project: Omit<ClientProject, 'related'>) => [
              project.id,
              {
                ...project,
                related: [],
              },
            ]),
          ),
        })
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
      refGeojson.features.slice().sort((a, b) => {
        const aName = a.properties?.[nameProp]
        const bName = b.properties?.[nameProp]
        return typeof aName === 'string' && typeof bName === 'string'
          ? aName.localeCompare(bName)
          : 0
      }),
    [nameProp],
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
          Type
        </Typography>
      </Grid2>
      <Grid2 xs={5} mt={1}>
        <Typography variant="h6" align="center">
          Projects
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
          value={allType}
          size="small"
          sx={{ width: '80%' }}
          onChange={(e) => {
            useStatic.setState((prev) => ({
              importWizard: {
                ...prev.importWizard,
                allType: e.target.value as typeof allType,
              },
            }))
            handleChange({
              ...geojson,
              features: geojson.features.map((feature) => ({
                ...feature,
                properties: {
                  ...feature.properties,
                  type: e.target.value ? (e.target.value as string) : undefined,
                },
              })),
            })
          }}
        >
          <MenuItem value="" />
          {(scannerType === 'rdm' ? RDM_FENCES : UNOWN_FENCES).map(
            (instanceType) => (
              <MenuItem key={instanceType} value={instanceType}>
                {instanceType}
              </MenuItem>
            ),
          )}
        </Select>
      </Grid2>
      <Grid2 xs={5} mt={1}>
        <ProjectsAc
          value={allProjects}
          setValue={(newValue) => {
            handleChange({
              ...geojson,
              features: geojson.features.map((feature) => ({
                ...feature,
                properties: {
                  ...feature.properties,
                  projects: newValue,
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
                      value={feature.properties?.type || ''}
                      onChange={(e) => {
                        const newFeature = {
                          ...feature,
                          properties: {
                            ...feature.properties,
                            type: e.target.value
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
                      {(scannerType === 'rdm' ? RDM_FENCES : UNOWN_FENCES).map(
                        (instanceType) => (
                          <MenuItem key={instanceType} value={instanceType}>
                            {instanceType}
                          </MenuItem>
                        ),
                      )}
                    </Select>
                  </Grid2>
                  <Grid2 xs={5}>
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
