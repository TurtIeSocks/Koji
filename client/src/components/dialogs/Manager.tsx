/* eslint-disable no-console */
import * as React from 'react'
import { Button, Dialog, DialogActions, DialogContent } from '@mui/material'
import type { FeatureCollection } from 'geojson'
import useDeepCompareEffect from 'use-deep-compare-effect'

import SplitMultiPolygonsBtn from '@components/buttons/SplitMultiPolygons'
import SaveToKoji from '@components/buttons/SaveToKoji'
import SaveToScanner from '@components/buttons/SaveToScanner'
import { safeParse } from '@services/utils'

import DialogHeader from './Header'
import { Code } from '../Code'

interface Props {
  open: string
  setOpen: (open: string) => void
  geojson: FeatureCollection
}

export default function Manager({ open, setOpen, geojson }: Props) {
  const [code, setCode] = React.useState<string>(
    JSON.stringify(geojson, null, 2),
  )

  useDeepCompareEffect(() => {
    setCode(JSON.stringify(geojson, null, 2))
  }, [geojson])

  const parsed = safeParse<FeatureCollection>(code)
  const safe = parsed.error ? geojson : parsed.value

  return (
    <Dialog open={open === 'rawManager'} fullScreen onClose={() => setOpen('')}>
      <DialogHeader action={() => setOpen('')}>Manager</DialogHeader>
      <DialogContent sx={{ margin: 0, padding: 0 }}>
        <Code code={code} setCode={setCode} />
      </DialogContent>
      <DialogActions>
        <SplitMultiPolygonsBtn
          fc={safe}
          setter={(fc) => setCode(JSON.stringify(fc))}
        />
        <SaveToKoji fc={code} />
        <SaveToScanner fc={code} />
        <Button
          onClick={() => {
            setOpen('')
          }}
        >
          Close
        </Button>
      </DialogActions>
    </Dialog>
  )
}
