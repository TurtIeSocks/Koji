import * as React from 'react'
import type { PopupProps } from '@assets/types'
import { Button, Typography } from '@mui/material'
import { useShapes } from '@hooks/useShapes'

interface Props extends Omit<PopupProps, 'dbRef'> {
  dis: number
}

export function LineStringPopup({ id, dis }: Props) {
  return (
    <>
      {process.env.NODE_ENV === 'development' && id}
      <br />
      <Typography align="center">{dis.toFixed(2)}m</Typography>
      <Button
        size="small"
        onClick={() => {
          if (id !== undefined) {
            useShapes.getState().setters.splitLine(id)
          }
        }}
      >
        Split
      </Button>
    </>
  )
}

export const MemoLinePopup = React.memo(
  LineStringPopup,
  (prev, next) => prev.dis === next.dis,
)
