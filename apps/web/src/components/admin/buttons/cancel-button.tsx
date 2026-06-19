import type { Ref } from "react";
import { CircleX } from "lucide-react";
import { Translate } from "shadmin-core";
import { useNavigate } from "react-router";

import { Button } from "@/components/ui/button";

/**
 * A button that navigates back to the previous page.
 *
 * Commonly used in form toolbars alongside SaveButton.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/cancel-button CancelButton documentation}
 *
 * @example
 * import { CancelButton, SaveButton, SimpleForm } from '@/components/admin';
 *
 * const FormToolbar = () => (
 *   <div className="flex flex-row gap-2 justify-end">
 *     <CancelButton />
 *     <SaveButton />
 *   </div>
 * );
 *
 * const PostEdit = () => (
 *   <Edit>
 *     <SimpleForm toolbar={<FormToolbar />}>
 *       ...
 *     </SimpleForm>
 *   </Edit>
 * );
 */
type CancelButtonProps = React.ComponentProps<"button"> & {
  ref?: Ref<HTMLButtonElement>;
};

function CancelButton({ ref, ...props }: CancelButtonProps) {
  const navigate = useNavigate();
  return (
    <Button
      type="button"
      variant="ghost"
      onClick={() => navigate(-1)}
      className="cursor-pointer"
      ref={ref}
      {...props}
    >
      <CircleX />
      <Translate i18nKey="ra.action.cancel">Cancel</Translate>
    </Button>
  );
}

export { CancelButton, type CancelButtonProps };
