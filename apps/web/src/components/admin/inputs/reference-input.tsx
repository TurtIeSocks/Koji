import type { ReactNode } from "react";
import type { ReferenceInputBaseProps } from "shadmin-core";
import { ReferenceInputBase } from "shadmin-core";
import { AutocompleteInput } from "@/components/admin/inputs/autocomplete-input";
import { Offline } from "@/components/admin/feedback/offline";

const defaultOffline = <Offline />;

interface ReferenceInputProps extends ReferenceInputBaseProps {
  /**
   * Call validate on the child component instead
   */
  validate?: never;
  /**
   * Component to display when offline and the request is pending or has stale placeholder data.
   */
  offline?: ReactNode;
}

/**
 * Form input for editing foreign key relationships with autocompletion.
 *
 * This component fetches related records from a reference resource and displays them in a searchable dropdown using AutocompleteInput.
 * Use it to edit many-to-one relationships, where the current record has a foreign key to another resource.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/reference-input ReferenceInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, TextInput, ReferenceInput } from '@/components/admin';
 *
 * const ContactEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="first_name" />
 *       <TextInput source="last_name" />
 *       <TextInput source="title" />
 *       <ReferenceInput source="company_id" reference="companies" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function ReferenceInput(props: ReferenceInputProps) {
  const {
    children = defaultChildren,
    offline = defaultOffline,
    ...rest
  } = props;

  if (props.validate && process.env.NODE_ENV !== "production") {
    throw new Error(
      "<ReferenceInput> does not accept a validate prop. Set the validate prop on the child instead.",
    );
  }

  return (
    <ReferenceInputBase {...rest} offline={offline}>
      {children}
    </ReferenceInputBase>
  );
}

const defaultChildren = <AutocompleteInput />;

export { ReferenceInput, type ReferenceInputProps };
