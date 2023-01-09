import * as React from 'react'
import { useStatic } from '@hooks/useStatic'
import Stepper from '@mui/material/Stepper'
import Step from '@mui/material/Step'
import StepLabel from '@mui/material/StepLabel'
import { Code } from '@components/Code'

import { Box, Button, Tab, Tabs } from '@mui/material'
import type { FeatureCollection } from 'geojson'
import { safeParse } from '@services/utils'

import BaseDialog from '../Base'
import ImportStep from './ImportStep'
import PropsStep from './PropsStep'
import AssignStep from './AssignStep'
import FinishStep from './Finish'
import TabPanel from './TabPanel'
import MiniMap from './MiniMap'

export default function ImportWizard() {
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

  const handleChange = (
    newGeojson: FeatureCollection,
    updatedSelected = false,
  ) => {
    if (updatedSelected) {
      useStatic.setState({
        importWizard: {
          ...importWizard,
          scannerSelected: new Set([
            ...importWizard.scannerSelected,
            ...newGeojson.features.map(
              (feat) => `${feat.properties?.name}__${feat.properties?.type}`,
            ),
          ]),
        },
      })
    }
    setTempGeojson((prev) => {
      const merged = {
        ...prev,
        ...newGeojson,
        features: [
          ...prev.features,
          ...newGeojson.features.map((feat) => ({
            ...feat,
            id: crypto.randomUUID(),
          })),
        ],
      }
      setCode(JSON.stringify(merged, null, 2))
      return merged
    })
  }

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

  const reset = (open = false) => {
    setStatic('importWizard', {
      open,
      nameProp: 'name',
      props: [],
      customName: '',
      modifier: 'none',
      allProjects: [],
      allType: '',
      checked: {},
      scannerSelected: new Set(),
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
          (feat) => importWizard.checked[feat.properties?.name],
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
          0: <ImportStep ref={tabRef} handleChange={handleChange} />,
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
          3: <FinishStep code={code} filtered={filtered} reset={reset} />,
        }[step] || null}
      </TabPanel>
      <TabPanel value={tab} index={1}>
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
      </TabPanel>
      <TabPanel value={tab} index={2}>
        <MiniMap filtered={filtered} />
      </TabPanel>
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
    </BaseDialog>
  )
}
