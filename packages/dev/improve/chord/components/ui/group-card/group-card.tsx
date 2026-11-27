import { ChevronRight } from "lucide-react";
import { useId, useState, type HTMLAttributes, type ReactNode } from "react";

/**
 * A group of rows in one rounded card, like Homerow's app groups: the header sits inside the
 * card, with an optional icon, a title and subtitle, a count, actions, and a disclosure chevron
 * that collapses the rows. A single hairline under the header is the card's only divider.
 */
export function GroupCard({
  leading,
  icon,
  title,
  subtitle,
  count,
  actions,
  listLabel,
  as: List = "div",
  dragging,
  open: controlledOpen,
  defaultOpen = true,
  onOpenChange,
  headerLabel,
  rootProps,
  className,
  children,
}: {
  /** A control before the header, such as a selection checkbox. */
  leading?: ReactNode;
  icon?: ReactNode;
  title: ReactNode;
  subtitle?: ReactNode;
  count?: number;
  /** Buttons beside the disclosure button, such as Edit Order or a menu. */
  actions?: ReactNode;
  listLabel?: string;
  /** `ul` when the rows are `li` elements. */
  as?: "div" | "ul";
  dragging?: boolean;
  /** Controls whether the rows show; leave unset for the card to keep its own state. */
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** An accessible name for the disclosure button, when the title alone isn't enough. */
  headerLabel?: string;
  /** Attributes for the card itself, such as data attributes a list uses to find it. */
  rootProps?: HTMLAttributes<HTMLDivElement> & Record<`data-${string}`, unknown>;
  /** Extra classes for the card, such as a tint for a warning. */
  className?: string;
  /** Leave out for a card that is only its header, with nothing to disclose. */
  children?: ReactNode;
}) {
  const [ownOpen, setOwnOpen] = useState(defaultOpen);
  const open = controlledOpen ?? ownOpen;
  const setOpen = (next: boolean) => {
    setOwnOpen(next);
    onOpenChange?.(next);
  };
  const listId = useId();
  const disclosable = children !== undefined && children !== null && children !== false;
  const heading = (
    <>
      {icon && (
        <span className="mac-icon" aria-hidden="true">
          {icon}
        </span>
      )}
      <span className="mac-row-text">
        <span className="mac-group-title">{title}</span>
        {subtitle && <span className="mac-row-subtitle">{subtitle}</span>}
      </span>
      {count !== undefined && <span className="mac-row-detail">{count}</span>}
    </>
  );
  return (
    <div
      {...rootProps}
      className={className ? `mac-list mac-group ${className}` : "mac-list mac-group"}
      data-open={(disclosable && open) || undefined}
      data-dragging={dragging || undefined}
    >
      <div className="mac-group-bar">
        {leading && <div className="mac-group-leading">{leading}</div>}
        <h3 className="min-w-0 flex-1">
          {disclosable ? (
            <button
              type="button"
              className="mac-group-header"
              aria-expanded={open}
              aria-controls={listId}
              aria-label={headerLabel}
              onClick={() => setOpen(!open)}
            >
              {heading}
              <ChevronRight
                className="mac-disclosure-chevron"
                size={14}
                strokeWidth={2.2}
                aria-hidden="true"
              />
            </button>
          ) : (
            <div className="mac-group-header">{heading}</div>
          )}
        </h3>
        {actions && <div className="mac-group-actions">{actions}</div>}
      </div>
      {disclosable && (
        <List
          id={listId}
          className="mac-group-list"
          role={List === "div" ? "list" : undefined}
          aria-label={listLabel}
          hidden={!open}
        >
          {open && children}
        </List>
      )}
    </div>
  );
}
