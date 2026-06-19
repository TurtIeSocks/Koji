import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
} from "@/components/admin";
import { PropertyValueInput } from "@/components/inputs/property-value-input";
import { PROPERTY_CATEGORIES } from "@/lib/constants";
import { required } from "ra-core";

export const PropertyFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <SelectInput
      source="category"
      choices={[...PROPERTY_CATEGORIES]}
      defaultValue="string"
      validate={required()}
    />
    <PropertyValueInput source="default_value" categorySource="category" />
  </>
);

export const PropertyCreate = () => (
  <Create>
    <SimpleForm>
      <PropertyFormFields />
    </SimpleForm>
  </Create>
);
