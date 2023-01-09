import * as React from 'react'
import { Button } from '@mui/material'
import Add from '@mui/icons-material/Add'
import ImportWizard from '@components/dialogs/import/ImportWizard'
import { useStatic } from '@hooks/useStatic'

export default function GeofenceCreateButton({
  children,
}: {
  children?: string
}) {
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
        {children ?? 'Create'}
      </Button>
      <ImportWizard />
    </>
  )
}
