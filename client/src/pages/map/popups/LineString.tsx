import * as React from 'react'
import type { PopupProps } from '@assets/types'

interface Props extends PopupProps {
  dis: number
}

export function LineStringPopup({ dis }: Props) {
  return <>{dis.toFixed(2)}</>
}

export const MemoLinePopup = React.memo(
  LineStringPopup,
  (prev, next) => prev.dis === next.dis,
)
