import {
  Create,
  SimpleForm,
  TextInput,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import { required } from "ra-core";

export const ProjectFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" label="Description" multiline />
    <ReferenceArrayInput source="geofences" reference="geofence">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
  </>
);

export const ProjectCreate = () => (
  <Create>
    <SimpleForm>
      <ProjectFormFields />
    </SimpleForm>
  </Create>
);
