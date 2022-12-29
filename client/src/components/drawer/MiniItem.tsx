import * as React from 'react'
import { Divider, ListItemButton, Tooltip } from '@mui/material'

import { ICON_MAP } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'

import { MySlide } from '../styled/Slide'

interface Props {
  text: string
  i: number
}

export default function MiniItem({ text, i }: Props) {
  const setStore = usePersist((s) => s.setStore)

  const Icon = ICON_MAP[text] || null

  return (
    <Tooltip
      title={text}
      enterDelay={0}
      enterTouchDelay={10}
      placement="right"
      TransitionComponent={MySlide}
      color="primary"
      arrow
    >
      <ListItemButton
        onClick={() => {
          setStore('drawer', true)
          setStore('menuItem', text)
        }}
        sx={(theme) => ({
          '&:hover': { color: theme.palette.primary.main },
          transition: '0.25s ease',
        })}
      >
        {!!i && <Divider />} <Icon fontSize="large" />
      </ListItemButton>
    </Tooltip>
  )
}
