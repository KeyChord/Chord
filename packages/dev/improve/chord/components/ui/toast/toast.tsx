import { Toast, toast } from "@heroui/react/toast";

function Toaster() {
  return <Toast.Provider placement="top end" />;
}
export { Toaster, toast };
