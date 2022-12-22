import * as React from 'react'
import {
  ListItem,
  ListItemIcon,
  ListItemText,
  Accordion,
  AccordionSummary,
  AccordionDetails,
} from '@mui/material'
import ExpandMoreIcon from '@mui/icons-material/ExpandMore'

import { ICON_MAP } from '@assets/constants'
import { useStore } from '@hooks/useStore'

interface Props {
  name: string
  children: React.ReactNode
}

export default function MenuAccordion({ name, children }: Props) {
  const [menuItem, setStore] = useStore((s) => [s.menuItem, s.setStore])

  const Icon = ICON_MAP[name] || null

  return (
    <ListItem key={name} disablePadding sx={{ display: 'block' }}>
      <Accordion
        expanded={menuItem === name}
        onChange={(_, isExpanded) => {
          setStore('menuItem', isExpanded ? name : '')
        }}
        TransitionProps={{ unmountOnExit: true }}
      >
        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
          <ListItemIcon>
            <Icon />
          </ListItemIcon>
          <ListItemText primary={name} />
        </AccordionSummary>
        <AccordionDetails>{children}</AccordionDetails>
      </Accordion>
    </ListItem>
  )
}
