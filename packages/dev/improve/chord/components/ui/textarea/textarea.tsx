import { cn } from "@chord/com.npmjs.utils-cn";
import { TextArea } from "@heroui/react/textarea";
import type { ComponentProps } from "react";

function Textarea({
  className,
  ...props
}: Omit<ComponentProps<typeof TextArea>, "className"> & { className?: string }) {
  return (
    <TextArea
      {...props}
      data-slot="textarea"
      className={cn("min-h-16 w-full rounded-lg px-2.5 py-2 text-sm", className)}
    />
  );
}
export { Textarea };
