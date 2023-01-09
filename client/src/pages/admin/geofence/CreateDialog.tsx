import * as React from 'react'
import { Button } from '@mui/material'
import Add from '@mui/icons-material/Add'
import ImportWizard from '@components/dialogs/import/ImportWizard'
import { useStatic } from '@hooks/useStatic'

export default function GeofenceCreateButton() {
  return (
    <>
      <Button
        color="primary"
        onClick={() =>
          useStatic.setState((prev) => ({
            importWizard: { ...prev.importWizard, open: true },
          }))
        }
      >
        <Add />
        Create
      </Button>
      <ImportWizard />
    </>
  )
}
