import { AlertDialog as HeroAlertDialog } from "@heroui/react/alert-dialog";
import { Description } from "@heroui/react/description";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import type { ComponentProps } from "react";

function AlertDialog({
  open,
  ...props
}: Omit<ComponentProps<typeof HeroAlertDialog>, "isOpen"> & { open?: boolean }) {
  return <HeroAlertDialog {...props} isOpen={open} />;
}
function AlertDialogContent({
  children,
  size = "default",
  ...props
}: ComponentProps<typeof HeroAlertDialog.Dialog> & { size?: "default" | "sm" }) {
  return (
    <HeroAlertDialog.Backdrop>
      <HeroAlertDialog.Container size={size === "sm" ? "sm" : "md"}>
        <HeroAlertDialog.Dialog {...props}>{children}</HeroAlertDialog.Dialog>
      </HeroAlertDialog.Container>
    </HeroAlertDialog.Backdrop>
  );
}
const AlertDialogHeader = HeroAlertDialog.Header;
const AlertDialogTitle = HeroAlertDialog.Heading;
const AlertDialogFooter = HeroAlertDialog.Footer;
function AlertDialogDescription(props: ComponentProps<typeof Description>) {
  return <Description {...props} slot="description" />;
}
function AlertDialogCancel(props: ComponentProps<typeof Button>) {
  return <Button variant="outline" {...props} slot="close" autoFocus />;
}
function AlertDialogAction(props: ComponentProps<typeof Button>) {
  return <Button {...props} slot="close" />;
}
export {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogCancel,
  AlertDialogAction,
};
