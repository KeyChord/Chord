import { cn } from "@chord/com.npmjs.utils-cn";
import { Card as HeroCard } from "@heroui/react/card";
import type { ComponentProps } from "react";

function Card({
  className,
  size = "default",
  children,
  ...props
}: ComponentProps<"div"> & { size?: "default" | "sm" }) {
  return (
    <HeroCard
      {...props}
      data-slot="card"
      data-size={size}
      className={cn("gap-4 rounded-xl text-sm", size === "sm" && "gap-3", className)}
    >
      {children}
    </HeroCard>
  );
}
function CardHeader(props: ComponentProps<"div">) {
  return <HeroCard.Header {...props} data-slot="card-header" />;
}
function CardTitle({ className, ...props }: ComponentProps<"h3">) {
  return (
    <HeroCard.Title
      {...props}
      data-slot="card-title"
      className={cn("text-base font-medium", className)}
    />
  );
}
function CardDescription({ className, ...props }: ComponentProps<"p">) {
  return (
    <HeroCard.Description
      {...props}
      data-slot="card-description"
      className={cn("text-sm text-muted-foreground", className)}
    />
  );
}
function CardContent(props: ComponentProps<"div">) {
  return <HeroCard.Content {...props} data-slot="card-content" />;
}
function CardFooter({ className, ...props }: ComponentProps<"div">) {
  return (
    <HeroCard.Footer
      {...props}
      data-slot="card-footer"
      className={cn("flex items-center gap-2", className)}
    />
  );
}
export { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter };
