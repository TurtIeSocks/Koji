import * as React from 'react'
import { TextField, Typography } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

import { KEYBOARD_SHORTCUTS } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'
import { useStatic } from '@hooks/useStatic'
import { fromCamelCase } from '@services/utils'

import BaseDialog from './Base'

export function KeyboardShortcuts() {
  const kbShortcuts = usePersist((s) => s.kbShortcuts)
  const open = useStatic((s) => s.dialogs.keyboard)

  const reverse = Object.fromEntries(
    Object.entries(kbShortcuts).map(([key, action]) => [action, key]),
  )
  return (
    <BaseDialog
      title="Set Keyboard Shortcuts"
      open={open}
      onClose={() =>
        useStatic.setState((prev) => ({
          dialogs: { ...prev.dialogs, keyboard: false },
        }))
      }
    >
      <Grid2 container>
        {KEYBOARD_SHORTCUTS.map(({ category, shortcuts }) => (
          <Grid2 key={category} xs={12} container>
            <Grid2 xs={12}>
              <Typography variant="h5" my={2}>
                {fromCamelCase(category)}
              </Typography>
            </Grid2>
            {shortcuts.map((key) => (
              <Grid2 key={key} container xs={12}>
                <Grid2 xs={5} sm={4} textAlign="left">
                  {fromCamelCase(key)}
                </Grid2>
                <Grid2 xs={7} sm={4} py={1}>
                  <TextField
                    size="small"
                    value={reverse[key] || ''}
                    onChange={() => {}}
                    onKeyUp={(e) => {
                      e.preventDefault()
                      let shortcut = ''
                      if (e.key.length > 1) return
                      if (e.key === 'Backspace') {
                        shortcut = ''
                        return
                      }
                      if (e.ctrlKey) shortcut += 'ctrl+'
                      if (e.altKey) shortcut += 'alt+'
                      if (e.shiftKey) shortcut += 'shift+'
                      shortcut += e.key.toLowerCase()
                      usePersist.setState((prev) => ({
                        kbShortcuts: {
                          ...prev.kbShortcuts,
                          [shortcut]: key,
                        },
                      }))
                    }}
                  />
                </Grid2>
              </Grid2>
            ))}
          </Grid2>
        ))}
      </Grid2>
    </BaseDialog>
  )
}
