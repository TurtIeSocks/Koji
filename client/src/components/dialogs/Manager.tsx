/* eslint-disable no-console */
import * as React from 'react'
import { Button, Dialog, DialogActions, DialogContent } from '@mui/material'
import type { FeatureCollection } from 'assets/types'
import useDeepCompareEffect from 'use-deep-compare-effect'
import shallow from 'zustand/shallow'

import SplitMultiPolygonsBtn from '@components/buttons/SplitMultiPolygons'
import SaveToKoji from '@components/buttons/SaveToKoji'
import SaveToScanner from '@components/buttons/SaveToScanner'
import { safeParse } from '@services/utils'
import { useStatic } from '@hooks/useStatic'

import DialogHeader from './Header'
import { Code } from '../Code'

export default function Manager() {
  const { dialogs, geojson } = useStatic((s) => s, shallow)
  const [code, setCode] = React.useState<string>(
    JSON.stringify(geojson, null, 2),
  )

  const setOpen = () =>
    useStatic.setState((prev) => ({
      dialogs: {
        ...prev.dialogs,
        manager: false,
      },
    }))

  useDeepCompareEffect(() => {
    if (dialogs.manager) setCode(JSON.stringify(geojson, null, 2))
  }, [geojson, dialogs.manager])

  const parsed = safeParse<FeatureCollection>(code)
  const safe = parsed.error ? geojson : parsed.value

  return (
    <Dialog open={dialogs.manager} fullScreen onClose={setOpen}>
      <DialogHeader action={setOpen}>Manager</DialogHeader>
      <DialogContent sx={{ margin: 0, padding: 0 }}>
        <Code code={code} setCode={setCode} />
      </DialogContent>
      <DialogActions>
        <SplitMultiPolygonsBtn
          fc={safe}
          setter={(fc) => setCode(JSON.stringify(fc))}
        />
        <SaveToKoji fc={safe} />
        <SaveToScanner fc={code} />
        <Button onClick={setOpen}>Close</Button>
      </DialogActions>
    </Dialog>
  )
}
