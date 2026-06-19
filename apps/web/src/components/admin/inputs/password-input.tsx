import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { useTranslate } from "shadmin-core";
import {
  TextInput,
  type TextInputProps,
} from "@/components/admin/inputs/text-input";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface PasswordInputProps
  extends Omit<TextInputProps, "type" | "multiline"> {
  initiallyVisible?: boolean;
}

/**
 * Text input for passwords, with a button to toggle visibility.
 *
 * Wraps `<TextInput>` and forces `type="password"`. Set `initiallyVisible` to
 * render the password in plain text on first render.
 *
 * @see {@link https://shadmin.turtlesocks.dev/docs/password-input PasswordInput documentation}
 *
 * @example
 * import { Edit, SimpleForm, PasswordInput } from '@/components/admin';
 *
 * const UserEdit = () => (
 *   <Edit>
 *     <SimpleForm>
 *       <PasswordInput source="password" />
 *     </SimpleForm>
 *   </Edit>
 * );
 */
function PasswordInput(props: PasswordInputProps) {
  const {
    initiallyVisible = false,
    className,
    inputClassName,
    ...rest
  } = props;
  const [visible, setVisible] = useState(initiallyVisible);
  const translate = useTranslate();
  const hasLabel = rest.label !== false;

  return (
    <div className={cn("relative", className)}>
      <TextInput
        {...rest}
        type={visible ? "text" : "password"}
        inputClassName={cn("pr-10", inputClassName)}
      />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className={cn(
          "absolute right-1 size-7 text-muted-foreground",
          hasLabel ? "top-6" : "top-1",
        )}
        onClick={() => setVisible((v) => !v)}
        aria-label={translate(
          visible
            ? "ra.input.password.toggle_visible"
            : "ra.input.password.toggle_hidden",
          { _: visible ? "Hide password" : "Show password" },
        )}
        tabIndex={-1}
      >
        {visible ? (
          <EyeOff className="size-4" aria-hidden="true" />
        ) : (
          <Eye className="size-4" aria-hidden="true" />
        )}
      </Button>
    </div>
  );
}

export { PasswordInput, type PasswordInputProps };
