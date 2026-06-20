import { Link } from "react-router";
import { Upload } from "lucide-react";
import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/** Toolbar link that opens the bulk import wizard at `/import`. */
function ImportButton() {
  return (
    <Link
      to="/import"
      className={cn(buttonVariants({ variant: "outline", size: "sm" }))}
    >
      <Upload className="size-4" />
      Import
    </Link>
  );
}

export { ImportButton };
