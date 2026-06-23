export const KANBAN_MIN_COLUMN_WIDTH = 300;
export const KANBAN_BOARD_GAP = 12;
export const KANBAN_GROUP_COLUMN_GAP = 10;
export const KANBAN_GROUP_INLINE_CHROME = 22;

export type KanbanLayoutGroupInput = {
  id: string;
  columns: number;
};

export type KanbanLayoutGroup = KanbanLayoutGroupInput & {
  width: number;
};

export type KanbanLayout = {
  columnWidth: number;
  groups: KanbanLayoutGroup[];
  scrolls: boolean;
  totalColumns: number;
};

export function calculateKanbanLayout({
  availableWidth,
  groups,
  minColumnWidth = KANBAN_MIN_COLUMN_WIDTH,
  boardGap = KANBAN_BOARD_GAP,
  groupColumnGap = KANBAN_GROUP_COLUMN_GAP,
  groupInlineChrome = KANBAN_GROUP_INLINE_CHROME,
}: {
  availableWidth: number;
  groups: KanbanLayoutGroupInput[];
  minColumnWidth?: number;
  boardGap?: number;
  groupColumnGap?: number;
  groupInlineChrome?: number;
}): KanbanLayout {
  const normalizedGroups = groups.map((group) => ({
    ...group,
    columns: Math.max(0, Math.floor(group.columns)),
  }));
  const totalColumns = normalizedGroups.reduce((sum, group) => sum + group.columns, 0);

  if (!Number.isFinite(availableWidth) || availableWidth <= 0 || totalColumns === 0) {
    return {
      columnWidth: minColumnWidth,
      groups: normalizedGroups.map((group) => ({
        ...group,
        width: groupWidth(group.columns, minColumnWidth, groupColumnGap, groupInlineChrome),
      })),
      scrolls: totalColumns > 0,
      totalColumns,
    };
  }

  const visibleGroups = normalizedGroups.filter((group) => group.columns > 0).length;
  const boardGapTotal = Math.max(0, visibleGroups - 1) * boardGap;
  const groupChromeTotal = visibleGroups * groupInlineChrome;
  const groupColumnGapTotal = normalizedGroups.reduce(
    (sum, group) => sum + Math.max(0, group.columns - 1) * groupColumnGap,
    0,
  );
  const availableForColumns =
    availableWidth - boardGapTotal - groupChromeTotal - groupColumnGapTotal;
  const fittedColumnWidth = Math.floor(availableForColumns / totalColumns);
  const columnWidth = Math.max(minColumnWidth, fittedColumnWidth);

  return {
    columnWidth,
    groups: normalizedGroups.map((group) => ({
      ...group,
      width: groupWidth(group.columns, columnWidth, groupColumnGap, groupInlineChrome),
    })),
    scrolls: fittedColumnWidth < minColumnWidth,
    totalColumns,
  };
}

function groupWidth(
  columns: number,
  columnWidth: number,
  groupColumnGap: number,
  groupInlineChrome: number,
) {
  if (columns <= 0) return groupInlineChrome;
  return columns * columnWidth + Math.max(0, columns - 1) * groupColumnGap + groupInlineChrome;
}
