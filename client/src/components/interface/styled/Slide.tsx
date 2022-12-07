import * as React from 'react'
import Slide, { type SlideProps } from '@mui/material/Slide'
import type { TransitionProps } from '@mui/material/transitions'

export const MySlide = React.forwardRef(
  (props: TransitionProps & SlideProps, ref: React.Ref<unknown>) => (
    <Slide direction="right" ref={ref} {...props} />
  ),
)
