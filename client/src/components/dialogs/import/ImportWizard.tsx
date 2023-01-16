import * as React from 'react'
import { useStatic } from '@hooks/useStatic'
import Stepper from '@mui/material/Stepper'
import Step from '@mui/material/Step'
import StepLabel from '@mui/material/StepLabel'
import { Code } from '@components/Code'

import { Box, Button, Tab, Tabs, Typography } from '@mui/material'
import type { FeatureCollection } from 'geojson'
import { safeParse } from '@services/utils'
import CombineByName from '@components/buttons/CombineByName'
import SplitMultiPolygonsBtn from '@components/buttons/SplitMultiPolygons'

import BaseDialog from '../Base'
import ImportStep from './ImportStep'
import PropsStep from './PropsStep'
import AssignStep from './AssignStep'
import FinishStep from './Finish'
import TabPanel from './TabPanel'
import MiniMap from './MiniMap'

export default function ImportWizard({ onClose }: { onClose?: () => void }) {
  const importWizard = useStatic((s) => s.importWizard)
  const setStatic = useStatic((s) => s.setStatic)

  const [tempGeojson, setTempGeojson] = React.useState<FeatureCollection>({
    type: 'FeatureCollection',
    bbox: [0, 0, 0, 0],
    features: [],
  })
  const [code, setCode] = React.useState(JSON.stringify(tempGeojson, null, 2))
  const [tab, setTab] = React.useState(0)
  const [step, setStep] = React.useState(0)

  const tabRef = React.useRef<HTMLDivElement>(null)

  const handleCodeChange = (newGeojson: FeatureCollection) => {
    setCode(
      JSON.stringify(
        {
          ...newGeojson,
          features: newGeojson.features.map((feat) => ({
            ...feat,
            properties: Object.fromEntries(
              Object.entries(feat.properties || {}).map(([k, v]) =>
                k.startsWith('__') ? [k, undefined] : [k, v],
              ),
            ),
          })),
        },
        null,
        2,
      ),
    )
  }

  const handleChange = (newGeojson: FeatureCollection, key = '') => {
    setTempGeojson((prev) => {
      const merged = {
        ...prev,
        ...newGeojson,
        features: [
          ...(key
            ? prev.features.filter(
                (feat) => feat.properties?.[key] === undefined,
              )
            : prev.features),
          ...newGeojson.features.map((feat) => ({
            ...feat,
            id: crypto.randomUUID(),
          })),
        ],
      }
      handleCodeChange(merged)
      return merged
    })
  }

  const reset = (open = false) => {
    if (onClose) {
      onClose()
    }
    setStatic('importWizard', {
      open,
      nameProp: '',
      props: [],
      customName: '',
      modifier: 'none',
      allProjects: [],
      allType: '',
      checked: {},
    })
    setTempGeojson({
      type: 'FeatureCollection',
      bbox: [0, 0, 0, 0],
      features: [],
    })
    setCode(
      '{ "type": "FeatureCollection", "features": [], "bbox": [0, 0, 0, 0] }',
    )
    setTab(0)
    setStep(0)
  }

  const parsed = safeParse<FeatureCollection>(code)
  const safe: FeatureCollection = parsed.error
    ? {
        type: 'FeatureCollection',
        bbox: [0, 0, 0, 0],
        features: [],
      }
    : parsed.value

  const filtered = Object.keys(importWizard.checked).length
    ? {
        ...safe,
        features: safe.features.filter(
          (feat) => importWizard.checked[feat.id || ''],
        ),
      }
    : safe

  return (
    <BaseDialog
      title="Import Wizard"
      open={importWizard.open}
      onClose={() => reset()}
      Components={{
        Dialog: {
          maxWidth: 'xl',
        },
        DialogContent: {
          sx: {
            m: 0,
            p: 0,
            minWidth: '80vw',
            overflow: 'auto',
          },
        },
        DialogActions: {
          children: (
            <>
              <Button color="error" onClick={() => reset(true)}>
                Reset
              </Button>
              <CombineByName
                fc={tempGeojson}
                nameProp={importWizard.nameProp}
                setter={(newFc) => {
                  setTempGeojson(newFc)
                  handleCodeChange(newFc)
                }}
                disabled={
                  !tempGeojson.features.length || !importWizard.nameProp
                }
              />
              <SplitMultiPolygonsBtn
                fc={tempGeojson}
                setter={(newFc) => {
                  setTempGeojson(newFc)
                  handleCodeChange(newFc)
                }}
                disabled={
                  !tempGeojson.features.length || !importWizard.nameProp
                }
              />
              <Box sx={{ flex: '1 1 auto' }} />
              <Button
                color="secondary"
                disabled={step === 0}
                onClick={() => {
                  setStep((prev) => prev - 1)
                }}
                sx={{ mr: 1 }}
              >
                Back
              </Button>
              <Button
                color="primary"
                disabled={step === 3 || !tempGeojson.features.length}
                onClick={() => {
                  setStep((prev) => prev + 1)
                  if (tab) setTab(0)
                }}
              >
                Next
              </Button>
            </>
          ),
        },
      }}
    >
      <Tabs
        value={tab}
        onChange={(_, newTab: number) => setTab(newTab)}
        textColor="primary"
        indicatorColor="primary"
        variant="fullWidth"
      >
        {['select', 'code', 'preview'].map((label, i) => (
          <Tab key={label} value={i} label={label} />
        ))}
      </Tabs>
      <TabPanel value={tab} index={0}>
        {{
          0: (
            <ImportStep
              ref={tabRef}
              geojson={tempGeojson}
              handleChange={handleChange}
            />
          ),
          1: (
            <PropsStep
              ref={tabRef}
              handleChange={handleCodeChange}
              geojson={tempGeojson}
            />
          ),
          2: (
            <AssignStep
              ref={tabRef}
              handleChange={handleCodeChange}
              geojson={safe}
              refGeojson={tempGeojson}
            />
          ),
          3: <FinishStep filtered={filtered} reset={reset} />,
        }[step] || null}
        <Stepper
          activeStep={step}
          alternativeLabel
          sx={{ mt: 3, maxWidth: '100%' }}
        >
          {['Import', 'Properties', 'Assign', 'Confirm'].map((label) => (
            <Step key={label}>
              <StepLabel>{label}</StepLabel>
            </Step>
          ))}
        </Stepper>
      </TabPanel>
      <TabPanel value={tab} index={1}>
        <Box py={3}>
          <Code
            code={
              Object.keys(importWizard.checked).length
                ? JSON.stringify(filtered, null, 2)
                : code
            }
            setCode={setCode}
            textMode={!code.startsWith('{') && !code.startsWith('[')}
            minHeight={`${tabRef.current?.clientHeight}px`}
            maxHeight="70vh"
          />
          <Typography variant="caption" color="grey">
            You can also try entering a url for a remote JSON, Kōji will attempt
            to fetch and parse it.
          </Typography>
        </Box>
      </TabPanel>
      <TabPanel value={tab} index={2}>
        <MiniMap filtered={filtered} />
      </TabPanel>
    </BaseDialog>
  )
}
