import { describe, expect, it } from "vitest";
import {
  KANBAN_MIN_COLUMN_WIDTH,
  calculateKanbanLayout,
  type KanbanLayoutGroupInput,
} from "./kanbanLayout";

const groups: KanbanLayoutGroupInput[] = [
  { id: "intake", columns: 1 },
  { id: "plan", columns: 2 },
  { id: "build", columns: 3 },
];

describe("calculateKanbanLayout", () => {
  it("expands columns to fill the available width when the board does not need scroll", () => {
    const layout = calculateKanbanLayout({ availableWidth: 2200, groups });

    expect(layout.totalColumns).toBe(6);
    expect(layout.columnWidth).toBeGreaterThan(KANBAN_MIN_COLUMN_WIDTH);
    expect(layout.scrolls).toBe(false);
  });

  it("uses the fixed minimum width when the board needs horizontal scroll", () => {
    const layout = calculateKanbanLayout({ availableWidth: 800, groups });

    expect(layout.columnWidth).toBe(KANBAN_MIN_COLUMN_WIDTH);
    expect(layout.scrolls).toBe(true);
  });

  it("sizes each group proportionally to its number of columns", () => {
    const layout = calculateKanbanLayout({ availableWidth: 2200, groups });
    const intake = layout.groups.find((group) => group.id === "intake");
    const plan = layout.groups.find((group) => group.id === "plan");
    const build = layout.groups.find((group) => group.id === "build");

    expect(intake?.width).toBeLessThan(plan?.width ?? 0);
    expect(plan?.width).toBeLessThan(build?.width ?? 0);
  });

  it("returns a safe fallback when the available width is invalid", () => {
    const layout = calculateKanbanLayout({ availableWidth: 0, groups });

    expect(layout.columnWidth).toBe(KANBAN_MIN_COLUMN_WIDTH);
    expect(layout.groups.every((group) => group.width > 0)).toBe(true);
  });
});
