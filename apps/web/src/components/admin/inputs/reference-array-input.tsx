import * as React from "react";
import type { ReactElement, ReactNode } from "react";
import type { InputProps, UseReferenceArrayInputParams } from "shadmin-core";
import {
  useReferenceArrayInputController,
  ResourceContextProvider,
  ChoicesContextProvider,
} from "shadmin-core";
import { AutocompleteArrayInput } from "@/components/admin/inputs/autocomplete-array-input";
import { Offline } from "@/components/admin/feedback/offline";

const defaultOffline = <Offline />;

interface ReferenceArrayInputProps
  extends InputProps,
    UseReferenceArrayInputParams {
  children?: ReactElement;
  /**
   * Component to display when offline and the request is pending or has stale placeholder data.
   */
  offline?: ReactNode;
}

/**
 * Form input for editing arrays of foreign key relationships with autocompletion.
 *
 * This component fetches related records from a reference resource and displays them
 * in a searchable multi-select interface using AutocompleteArrayInput.
 * Use it to edit one-to-many or many-to-many relationships, where the current record
 * has an array of foreign keys to another resource.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/reference-array-input ReferenceArrayInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, TextInput, ReferenceArrayInput } from '@/components/admin';
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <TextInput source="title" />
 *       <ReferenceArrayInput source="tag_ids" reference="tags" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function ReferenceArrayInput(props: ReferenceArrayInputProps) {
  const {
    children = defaultChildren,
    offline = defaultOffline,
    reference,
    sort,
    filter = defaultFilter,
  } = props;
  if (React.Children.count(children) !== 1) {
    throw new Error(
      "<ReferenceArrayInput> only accepts a single child (like <AutocompleteArrayInput>)",
    );
  }

  const controllerProps = useReferenceArrayInputController({
    ...props,
    sort,
    filter,
  });

  const { isPaused, isPending } = controllerProps;
  // Render offline placeholder when network is paused and choices are unavailable
  if (isPaused && isPending && offline !== undefined && offline !== false) {
    return <>{offline}</>;
  }

  return (
    <ResourceContextProvider value={reference}>
      <ChoicesContextProvider value={controllerProps}>
        {children}
      </ChoicesContextProvider>
    </ResourceContextProvider>
  );
}

const defaultChildren = <AutocompleteArrayInput />;
const defaultFilter = {};

export { ReferenceArrayInput, type ReferenceArrayInputProps };
