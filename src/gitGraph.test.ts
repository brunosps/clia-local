import { describe, expect, it } from "vitest";
import { computeGraph } from "./gitGraph";

describe("computeGraph", () => {
  it("keeps a linear history in column 0", () => {
    const layout = computeGraph([
      { sha: "a", parents: ["b"] },
      { sha: "b", parents: ["c"] },
      { sha: "c", parents: [] },
    ]);
    expect(layout.order.map((n) => n.column)).toEqual([0, 0, 0]);
    expect(layout.columns).toBe(1);
  });

  it("opens a second lane for a divergent branch and collapses at the shared parent", () => {
    // a and b both have parent c (two branch tips), newest first.
    const layout = computeGraph([
      { sha: "a", parents: ["c"] },
      { sha: "b", parents: ["c"] },
      { sha: "c", parents: [] },
    ]);
    expect(layout.nodes.get("a")!.column).toBe(0);
    expect(layout.nodes.get("b")!.column).toBe(1);
    expect(layout.nodes.get("c")!.column).toBe(0); // collapses back to lane 0
    expect(layout.columns).toBe(2);
  });

  it("places a merge commit's second parent in a new lane", () => {
    const layout = computeGraph([
      { sha: "m", parents: ["p1", "p2"] },
      { sha: "p1", parents: ["base"] },
      { sha: "p2", parents: ["base"] },
      { sha: "base", parents: [] },
    ]);
    expect(layout.nodes.get("m")!.column).toBe(0);
    expect(layout.nodes.get("p1")!.column).toBe(0);
    expect(layout.nodes.get("p2")!.column).toBe(1);
    expect(layout.nodes.get("base")!.column).toBe(0);
  });

  it("handles multiple independent roots", () => {
    const layout = computeGraph([
      { sha: "a", parents: [] },
      { sha: "b", parents: [] },
    ]);
    expect(layout.nodes.get("a")!.column).toBe(0);
    expect(layout.nodes.get("b")!.column).toBe(0); // lane freed after root, reused
    expect(layout.order).toHaveLength(2);
  });

  it("assigns distinct colors to distinct lanes", () => {
    const layout = computeGraph([
      { sha: "a", parents: ["c"] },
      { sha: "b", parents: ["c"] },
      { sha: "c", parents: [] },
    ]);
    expect(layout.nodes.get("a")!.color).not.toBe(layout.nodes.get("b")!.color);
  });
});
