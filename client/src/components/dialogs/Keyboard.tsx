import * as React from 'react'
import { Button, IconButton, TextField, Typography } from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import ClearIcon from '@mui/icons-material/Clear'

import { KEYBOARD_SHORTCUTS } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'
import { useStatic } from '@hooks/useStatic'
import { buildShortcutKey, fromCamelCase, reverseObject } from '@services/utils'

import BaseDialog from './Base'

export function KeyboardShortcuts() {
  const kbShortcuts = usePersist((s) => s.kbShortcuts)
  const open = useStatic((s) => s.dialogs.keyboard)

  return (
    <BaseDialog
      title="Set Keyboard Shortcuts"
      open={open}
      onClose={() =>
        useStatic.setState((prev) => ({
          dialogs: { ...prev.dialogs, keyboard: false },
        }))
      }
      Components={{
        DialogActions: {
          children: (
            <>
              <Button onClick={() => usePersist.setState({ kbShortcuts: {} })}>
                Reset
              </Button>
              <Grid2 flexGrow={1} />
            </>
          ),
        },
      }}
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
                    value={kbShortcuts[key] || ''}
                    onChange={() => {}}
                    onKeyUp={(e) => {
                      e.preventDefault()
                      if (e.key.length > 1) return
                      const shortcut = buildShortcutKey(e)
                      const reverse = reverseObject(kbShortcuts)
                      usePersist.setState((prev) => ({
                        kbShortcuts: {
                          ...prev.kbShortcuts,
                          [key]: shortcut,
                          [reverse[shortcut]]: prev.kbShortcuts[key],
                        },
                      }))
                    }}
                    InputProps={{
                      endAdornment: (
                        <IconButton
                          sx={{ mr: -2 }}
                          onClick={() =>
                            usePersist.setState((prev) => {
                              const clean = structuredClone(prev.kbShortcuts)
                              delete clean[key]
                              return { kbShortcuts: clean }
                            })
                          }
                        >
                          <ClearIcon />
                        </IconButton>
                      ),
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
