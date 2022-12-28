import * as React from 'react'
import { Edit } from 'react-admin'

import ProjectForm from './ProjectForm'

export default function ProjectEdit() {
  return (
    <Edit mutationMode="pessimistic">
      <ProjectForm />
    </Edit>
  )
}
