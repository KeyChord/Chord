import { Modal } from "@heroui/react/modal";
import { Description } from "@heroui/react/description";
import type { ComponentProps } from "react";

function Dialog({
  open,
  onOpenChange,
  ...props
}: Omit<ComponentProps<typeof Modal>, "isOpen"> & { open?: boolean }) {
  return <Modal {...props} isOpen={open} onOpenChange={onOpenChange} />;
}
function DialogContent({
  children,
  showCloseButton = true,
  isOpen,
  onOpenChange,
  ...props
}: ComponentProps<typeof Modal.Dialog> &
  Pick<ComponentProps<typeof Modal.Backdrop>, "isOpen" | "onOpenChange"> & {
    showCloseButton?: boolean;
  }) {
  return (
    <Modal.Backdrop isOpen={isOpen} onOpenChange={onOpenChange}>
      <Modal.Container>
        <Modal.Dialog {...props}>
          {(values) => (
            <>
              {showCloseButton && <Modal.CloseTrigger aria-label="Close dialog" />}
              {typeof children === "function" ? children(values) : children}
            </>
          )}
        </Modal.Dialog>
      </Modal.Container>
    </Modal.Backdrop>
  );
}
const DialogHeader = Modal.Header;
const DialogTitle = Modal.Heading;
const DialogFooter = Modal.Footer;
function DialogDescription(props: ComponentProps<typeof Description>) {
  return <Description {...props} slot="description" />;
}
export { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter };
