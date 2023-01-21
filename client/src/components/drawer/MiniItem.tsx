import * as React from 'react'
import { Divider, ListItemButton, SvgIcon, Tooltip } from '@mui/material'

import { ICON_MAP, TABS } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'

import { MySlide } from '../styled/Slide'

interface Props {
  text: typeof TABS[number] | 'Open'
  i: number
}

export default function MiniItem({ text, i }: Props) {
  const setStore = usePersist((s) => s.setStore)

  const Icon = text === 'Open' ? null : ICON_MAP[text] || null

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
          if (text !== 'Open') {
            setStore('menuItem', text)
          }
        }}
        sx={(theme) => ({
          '&:hover': { color: theme.palette.primary.main },
          transition: '0.25s ease',
        })}
      >
        {!!i && <Divider />}
        {Icon ? (
          <Icon fontSize="large" />
        ) : (
          <SvgIcon fontSize="large">
            {/* Chevron right import seems to be broken... */}
            <path d="M10 6 8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
          </SvgIcon>
        )}
      </ListItemButton>
    </Tooltip>
  )
}
