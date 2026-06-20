import {
  Create,
  SimpleForm,
  TextInput,
  BooleanInput,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import { required } from "ra-core";

export const ProjectFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="api_endpoint" label="API endpoint" />
    <TextInput source="api_key" label="API key" />
    <TextInput source="description" label="Description" multiline />
    <BooleanInput source="golbat" label="Golbat Sync" defaultValue={false} />
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
