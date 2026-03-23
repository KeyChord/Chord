import { useEffect, useState } from "react";
import { hotkeysCoreFeature, syncDataLoaderFeature, type Updater } from "@headless-tree/core";
import { useTree } from "@headless-tree/react";
import { ChevronRight } from "lucide-react";
import { Badge } from "#/components/ui/badge.tsx";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "#/components/ui/collapsible.tsx";
import { AppIcon } from "#/components/settings/app-icon.tsx";
import { cn } from "#/utils/style.ts";
import {
  buildActiveChordTreeModel,
  buildChordTree,
  getAppLabel,
  type ActiveChordTreeItem,
  type AppMetadataByBundleId,
  type ChordGroup,
  type ChordTreeNode,
} from "#/utils/settings.ts";

function getTreeGuide(ancestorHasNextSiblings: boolean[], isLast: boolean) {
  return `${ancestorHasNextSiblings.map((hasNext) => (hasNext ? "│  " : "   ")).join("")}${isLast ? "└─ " : "├─ "}`;
}

function ChordTreeRows({
  nodes,
  ancestorHasNextSiblings = [],
}: {
  nodes: ChordTreeNode[];
  ancestorHasNextSiblings?: boolean[];
}) {
  return (
    <div className="space-y-1">
      {nodes.map((node, index) => {
        const isLast = index + 1 === nodes.length;
        const guide = getTreeGuide(ancestorHasNextSiblings, isLast);
        const nextAncestors = [...ancestorHasNextSiblings, !isLast];

        return (
          <div key={node.key} className="space-y-1">
            <div className="grid grid-cols-[minmax(0,140px)_minmax(0,1fr)] gap-x-3 rounded-sm px-2.5 py-1 text-xs">
              <div className="truncate font-mono text-[11px] text-foreground/85">
                <span className="whitespace-pre text-muted-foreground">{guide}</span>
                <span className={node.chords.length === 0 ? "text-muted-foreground" : undefined}>
                  {node.prefix}
                </span>
              </div>
              <div className="min-w-0">
                {node.chords.length === 0 ? (
                  <span className="text-[11px] text-muted-foreground">
                    {node.chordCount} chords
                  </span>
                ) : (
                  <div className="space-y-1">
                    {node.chords.map((chord) => (
                      <div
                        key={`${chord.scopeKind}:${chord.scope}:${chord.sequence}:${chord.name}:${chord.action}`}
                        className="flex items-baseline gap-2"
                      >
                        <span className="truncate font-medium">{chord.name}</span>
                        <span className="truncate text-muted-foreground">{chord.action}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {node.children.length > 0 ? (
              <ChordTreeRows nodes={node.children} ancestorHasNextSiblings={nextAncestors} />
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

export function ChordGroupList({
  groups,
  forceOpen = false,
  appMetadataByBundleId,
  openGroups,
  onGroupOpenChange,
}: {
  groups: ChordGroup[];
  forceOpen?: boolean;
  appMetadataByBundleId: AppMetadataByBundleId;
  openGroups: Record<string, boolean>;
  onGroupOpenChange: (groupKey: string, open: boolean) => void;
}) {
  return (
    <div className="space-y-2">
      {groups.map((group) => {
        const isOpen = forceOpen || openGroups[group.key] === true;
        const tree = buildChordTree(group.chords);
        const appMetadata =
          group.scopeKind === "app" ? appMetadataByBundleId[group.scope] : undefined;
        const appLabel = getAppLabel(group.scope, appMetadata);
        const showsBundleId = group.scopeKind === "app" && appLabel !== group.scope;

        return (
          <Collapsible
            key={group.key}
            open={isOpen}
            onOpenChange={(open) => {
              onGroupOpenChange(group.key, open);
            }}
          >
            <CollapsibleTrigger asChild>
              <button
                type="button"
                className="flex w-full items-center gap-2 rounded-md border bg-background/80 px-2.5 py-1.5 text-left hover:bg-muted/70"
              >
                <ChevronRight
                  className={`size-3.5 shrink-0 transition-transform ${isOpen ? "rotate-90" : ""}`}
                />
                <Badge variant={group.scopeKind === "global" ? "secondary" : "outline"}>
                  {group.scopeKind === "global" ? "Global" : "App"}
                </Badge>
                {group.scopeKind === "app" ? (
                  <AppIcon appMetadata={appMetadata} label={appLabel} />
                ) : null}
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">
                    {group.scopeKind === "global" ? group.scope : appLabel}
                  </div>
                  {showsBundleId ? (
                    <div className="truncate text-xs text-muted-foreground">{group.scope}</div>
                  ) : null}
                </div>
                <span className="text-xs text-muted-foreground">{group.chords.length}</span>
              </button>
            </CollapsibleTrigger>

            <CollapsibleContent className="pt-1">
              <div className="overflow-hidden rounded-md border bg-background/80 px-1.5 py-2">
                <ChordTreeRows nodes={tree} />
              </div>
            </CollapsibleContent>
          </Collapsible>
        );
      })}
    </div>
  );
}

export function ActiveChordTree({
  groups,
  forceExpandAll = false,
  appMetadataByBundleId,
  openGroups,
  onGroupOpenChange,
}: {
  groups: ChordGroup[];
  forceExpandAll?: boolean;
  appMetadataByBundleId: AppMetadataByBundleId;
  openGroups: Record<string, boolean>;
  onGroupOpenChange: (groupKey: string, open: boolean) => void;
}) {
  const treeModel = buildActiveChordTreeModel(groups, appMetadataByBundleId);
  const defaultExpandedItems = forceExpandAll
    ? treeModel.folderItemIds
    : [
        ...treeModel.prefixFolderItemIds,
        ...treeModel.groupItemIds.filter((itemId) => {
          const groupKey = treeModel.groupKeyByItemId[itemId];
          return openGroups[groupKey] === true;
        }),
      ];
  const defaultExpandedSignature = defaultExpandedItems.join("\u001f");
  const [expandedItems, setExpandedItems] = useState<string[]>(defaultExpandedItems);

  useEffect(() => {
    setExpandedItems(defaultExpandedItems);
  }, [defaultExpandedSignature]);

  const tree = useTree<ActiveChordTreeItem>({
    rootItemId: treeModel.rootItemId,
    getItemName: (item) => item.getItemData().label,
    isItemFolder: (item) => item.getItemData().childIds.length > 0,
    dataLoader: {
      getItem: (itemId) => {
        const item = treeModel.itemsById[itemId];
        if (!item) {
          throw new Error(`Missing active chord tree item: ${itemId}`);
        }

        return item;
      },
      getChildren: (itemId) => treeModel.itemsById[itemId]?.childIds ?? [],
    },
    state: { expandedItems },
    setExpandedItems: (updaterOrValue: Updater<string[]>) => {
      setExpandedItems((current) => {
        const next =
          typeof updaterOrValue === "function" ? updaterOrValue(current) : updaterOrValue;
        const nextSet = new Set(next);

        for (const groupItemId of treeModel.groupItemIds) {
          const groupKey = treeModel.groupKeyByItemId[groupItemId];
          onGroupOpenChange(groupKey, nextSet.has(groupItemId));
        }

        return next;
      });
    },
    features: [syncDataLoaderFeature, hotkeysCoreFeature],
  });

  return (
    <div className="overflow-hidden rounded-md border bg-background/80">
      <div
        {...tree.getContainerProps("Registered chords")}
        className="divide-y divide-border/60 outline-none"
      >
        {tree.getItems().map((item) => {
          const data = item.getItemData();

          return (
            <div
              {...item.getProps()}
              key={item.getId()}
              className={cn(
                "flex items-start gap-2 px-2.5 py-2 outline-none transition-colors",
                "hover:bg-muted/60",
                item.isFocused() ? "bg-accent/70" : undefined,
              )}
              style={{ paddingLeft: `${item.getItemMeta().level * 18 + 10}px` }}
            >
              <span className="mt-0.5 flex size-4 shrink-0 items-center justify-center text-muted-foreground">
                {data.childIds.length > 0 ? (
                  <ChevronRight
                    className={cn(
                      "size-3.5 transition-transform",
                      item.isExpanded() ? "rotate-90" : undefined,
                    )}
                  />
                ) : (
                  <span className="size-1.5 rounded-full bg-border" />
                )}
              </span>

              {data.kind === "group" ? (
                <div className="flex min-w-0 flex-1 items-start gap-2">
                  <Badge variant={data.scopeKind === "global" ? "secondary" : "outline"}>
                    {data.scopeKind === "global" ? "Global" : "App"}
                  </Badge>
                  {data.scopeKind === "app" && data.appLabel ? (
                    <AppIcon appMetadata={data.appMetadata} label={data.appLabel} />
                  ) : null}
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">
                      {data.scopeKind === "global" ? data.scope : data.appLabel}
                    </div>
                    {data.showsBundleId ? (
                      <div className="truncate text-xs text-muted-foreground">{data.scope}</div>
                    ) : null}
                  </div>
                  <span className="shrink-0 text-xs text-muted-foreground">{data.chordCount}</span>
                </div>
              ) : (
                <div className="grid min-w-0 flex-1 grid-cols-[minmax(0,140px)_minmax(0,1fr)] gap-x-3">
                  <div className="truncate font-mono text-[11px] text-foreground/85">
                    <span
                      className={data.chords.length === 0 ? "text-muted-foreground" : undefined}
                    >
                      {data.prefix}
                    </span>
                  </div>
                  <div className="min-w-0">
                    {data.chords.length === 0 ? (
                      <span className="text-[11px] text-muted-foreground">
                        {data.chordCount} chords
                      </span>
                    ) : (
                      <div className="space-y-1">
                        {data.chords.map((chord) => (
                          <div
                            key={`${chord.scopeKind}:${chord.scope}:${chord.sequence}:${chord.name}:${chord.action}`}
                            className="flex items-baseline gap-2"
                          >
                            <span className="truncate font-medium">{chord.name}</span>
                            <span className="truncate text-muted-foreground">{chord.action}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
