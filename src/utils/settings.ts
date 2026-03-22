import type {
  ActiveChordInfo,
  AppMetadataInfo,
} from "#/api/settings.ts";

export type ChordGroup = {
  key: string;
  scope: string;
  scopeKind: "global" | "app";
  chords: ActiveChordInfo[];
};

type MutableChordTreeNode = {
  prefix: string;
  chords: ActiveChordInfo[];
  children: Map<string, MutableChordTreeNode>;
};

export type ChordTreeNode = {
  key: string;
  prefix: string;
  chords: ActiveChordInfo[];
  children: ChordTreeNode[];
  chordCount: number;
};

export type AppMetadataByBundleId = Record<string, AppMetadataInfo>;

export type ActiveChordTreeItem = {
  id: string;
  kind: "group" | "prefix" | "root";
  label: string;
  childIds: string[];
  chords: ActiveChordInfo[];
  chordCount: number;
  prefix?: string;
  groupKey?: string;
  scope?: string;
  scopeKind?: "global" | "app";
  appLabel?: string;
  showsBundleId?: boolean;
  appMetadata?: AppMetadataInfo;
};

export type ActiveChordTreeModel = {
  rootItemId: string;
  itemsById: Record<string, ActiveChordTreeItem>;
  groupItemIds: string[];
  groupKeyByItemId: Record<string, string>;
  folderItemIds: string[];
  prefixFolderItemIds: string[];
};

export function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function getAppLabel(
  bundleId: string,
  appMetadata?: AppMetadataInfo,
  fallbackDisplayName?: string | null,
) {
  return appMetadata?.displayName ?? fallbackDisplayName ?? bundleId;
}

export async function validateLocalChordFolder(path: string) {
  const fsApi = window.__TAURI__?.fs;
  if (!fsApi) {
    throw new Error("Filesystem plugin is not available.");
  }

  const exists = await fsApi.exists(path);
  if (!exists) {
    throw new Error("Selected folder is no longer available.");
  }

  const entries = await fsApi.readDir(path);
  const hasChordsDirectory = entries.some((entry) => entry.isDirectory && entry.name === "chords");
  if (!hasChordsDirectory) {
    throw new Error("Selected folder must contain a top-level chords directory.");
  }
}

function compareChordGroups(left: ChordGroup, right: ChordGroup) {
  if (left.scopeKind !== right.scopeKind) {
    return left.scopeKind === "global" ? -1 : 1;
  }

  return left.scope.localeCompare(right.scope);
}

export function buildChordGroups(chords: ActiveChordInfo[]): ChordGroup[] {
  const chordGroups: ChordGroup[] = [];
  const chordGroupMap = new Map<string, ChordGroup>();

  for (const chord of chords) {
    const key = `${chord.scopeKind}:${chord.scope}`;
    const existingGroup = chordGroupMap.get(key);
    if (existingGroup) {
      existingGroup.chords.push(chord);
      continue;
    }

    const group: ChordGroup = {
      key,
      scope: chord.scope,
      scopeKind: chord.scopeKind as "global" | "app",
      chords: [chord],
    };
    chordGroupMap.set(key, group);
    chordGroups.push(group);
  }

  chordGroups.sort(compareChordGroups);

  for (const group of chordGroups) {
    group.chords.sort(
      (left, right) =>
        left.sequence.localeCompare(right.sequence)
        || left.name.localeCompare(right.name)
        || left.action.localeCompare(right.action),
    );
  }

  return chordGroups;
}

function createMutableChordTreeNode(prefix: string): MutableChordTreeNode {
  return {
    prefix,
    chords: [],
    children: new Map(),
  };
}

function compressChordTreeNode(node: MutableChordTreeNode): ChordTreeNode {
  let current = node;

  while (current.chords.length === 0 && current.children.size === 1) {
    current = current.children.values().next().value as MutableChordTreeNode;
  }

  const children = [...current.children.values()]
    .sort((left, right) => left.prefix.localeCompare(right.prefix))
    .map((child) => compressChordTreeNode(child));
  const chordCount = current.chords.length + children.reduce((count, child) => count + child.chordCount, 0);

  return {
    key: current.prefix,
    prefix: current.prefix,
    chords: current.chords,
    children,
    chordCount,
  };
}

export function buildChordTree(chords: ActiveChordInfo[]): ChordTreeNode[] {
  const root = createMutableChordTreeNode("");

  for (const chord of chords) {
    let current = root;

    for (const key of chord.sequence) {
      let child = current.children.get(key);
      if (!child) {
        child = createMutableChordTreeNode(`${current.prefix}${key}`);
        current.children.set(key, child);
      }

      current = child;
    }

    current.chords.push(chord);
  }

  return [...root.children.values()]
    .sort((left, right) => left.prefix.localeCompare(right.prefix))
    .map((node) => compressChordTreeNode(node));
}

export function buildActiveChordTreeModel(
  groups: ChordGroup[],
  appMetadataByBundleId: AppMetadataByBundleId,
): ActiveChordTreeModel {
  const rootItemId = "__active-chords_root__";
  const itemsById: Record<string, ActiveChordTreeItem> = {
    [rootItemId]: {
      id: rootItemId,
      kind: "root",
      label: "Registered chords",
      childIds: [],
      chords: [],
      chordCount: groups.reduce((count, group) => count + group.chords.length, 0),
    },
  };
  const groupItemIds: string[] = [];
  const groupKeyByItemId: Record<string, string> = {};

  const registerPrefixNode = (groupKey: string, node: ChordTreeNode): string => {
    const itemId = `prefix:${groupKey}:${node.key}`;
    const childIds = node.children.map((child) => registerPrefixNode(groupKey, child));

    itemsById[itemId] = {
      id: itemId,
      kind: "prefix",
      label: node.prefix,
      childIds,
      chords: node.chords,
      chordCount: node.chordCount,
      prefix: node.prefix,
    };

    return itemId;
  };

  for (const group of groups) {
    const appMetadata = group.scopeKind === "app" ? appMetadataByBundleId[group.scope] : undefined;
    const appLabel = getAppLabel(group.scope, appMetadata);
    const showsBundleId = group.scopeKind === "app" && appLabel !== group.scope;
    const groupItemId = `group:${group.key}`;
    const childIds = buildChordTree(group.chords).map((node) => registerPrefixNode(group.key, node));

    itemsById[groupItemId] = {
      id: groupItemId,
      kind: "group",
      label: group.scopeKind === "global" ? group.scope : appLabel,
      childIds,
      chords: [],
      chordCount: group.chords.length,
      groupKey: group.key,
      scope: group.scope,
      scopeKind: group.scopeKind,
      appLabel,
      showsBundleId,
      appMetadata,
    };

    itemsById[rootItemId].childIds.push(groupItemId);
    groupItemIds.push(groupItemId);
    groupKeyByItemId[groupItemId] = group.key;
  }

  const folderItemIds = Object.values(itemsById)
    .filter((item) => item.kind !== "root" && item.childIds.length > 0)
    .map((item) => item.id);
  const prefixFolderItemIds = folderItemIds.filter((itemId) => itemsById[itemId]?.kind === "prefix");

  return {
    rootItemId,
    itemsById,
    groupItemIds,
    groupKeyByItemId,
    folderItemIds,
    prefixFolderItemIds,
  };
}
