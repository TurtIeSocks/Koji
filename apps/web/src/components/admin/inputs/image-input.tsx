import {
  FileInput,
  type FileInputProps,
} from "@/components/admin/inputs/file-input";
import { ImageField } from "@/components/admin/fields/image-field";

/**
 * Image upload input with drag-and-drop support and thumbnail previews.
 *
 * Use `<ImageInput>` for image attachments. It is a thin wrapper around
 * `<FileInput>` that defaults `accept` to `{ "image/*": [] }` and renders
 * an `<ImageField>` thumbnail preview when no child is provided.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/image-input ImageInput documentation}
 * @see {@link https://github.com/marmelab/react-admin/blob/master/packages/ra-ui-materialui/src/input/ImageInput.tsx ra-ui-materialui ImageInput}
 *
 * @example
 * import { Edit, SimpleForm, ImageInput, ImageField } from '@/components/admin';
 *
 * const ProductEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <ImageInput source="pictures" multiple>
 *         <ImageField source="src" title="title" />
 *       </ImageInput>
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function ImageInput(props: ImageInputProps) {
  const {
    accept = { "image/*": [] },
    labelMultiple = "ra.input.image.upload_several",
    labelSingle = "ra.input.image.upload_single",
    children,
    ...rest
  } = props;

  return (
    <FileInput
      accept={accept}
      labelMultiple={labelMultiple}
      labelSingle={labelSingle}
      {...rest}
    >
      {children ?? (
        <ImageField
          source="src"
          title="title"
          className="[&_img]:h-24 [&_img]:w-24 [&_img]:rounded-md [&_img]:object-cover"
        />
      )}
    </FileInput>
  );
}

type ImageInputProps = FileInputProps;

export { ImageInput, type ImageInputProps };
