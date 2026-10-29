import { Tabs as HeroTabs } from "@heroui/react/tabs";
import type { ComponentProps } from "react";
import { cn } from "@chord/com.npmjs.utils-cn";

function Tabs({
  value,
  defaultValue,
  onValueChange,
  ...props
}: Omit<
  ComponentProps<typeof HeroTabs>,
  "selectedKey" | "defaultSelectedKey" | "onSelectionChange"
> & {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
}) {
  return (
    <HeroTabs
      {...props}
      selectedKey={value}
      defaultSelectedKey={defaultValue}
      onSelectionChange={(key) => onValueChange?.(String(key))}
    />
  );
}
function TabsList(props: ComponentProps<typeof HeroTabs.List>) {
  return (
    <HeroTabs.ListContainer className="w-fit">
      <HeroTabs.List {...props} />
    </HeroTabs.ListContainer>
  );
}
function TabsTrigger({
  value,
  children,
  ...props
}: Omit<ComponentProps<typeof HeroTabs.Tab>, "id"> & { value: string }) {
  return (
    <HeroTabs.Tab {...props} id={value}>
      {(values) => (
        <>
          {typeof children === "function" ? children(values) : children}
          <HeroTabs.Indicator />
        </>
      )}
    </HeroTabs.Tab>
  );
}
function TabsContent({
  value,
  forceMount,
  hidden,
  className,
  ...props
}: Omit<ComponentProps<typeof HeroTabs.Panel>, "id"> & {
  value: string;
  forceMount?: boolean;
  hidden?: boolean;
}) {
  return (
    <HeroTabs.Panel
      {...props}
      id={value}
      shouldForceMount={forceMount}
      className={cn("data-[inert=true]:hidden", hidden && "hidden", className)}
    />
  );
}
export { Tabs, TabsList, TabsTrigger, TabsContent };
