// Pure commit-graph lane layout. Given commits in display order (newest first,
// as `git log --date-order` returns), assign each commit a column ("lane") and a
// color. The renderer draws a node per commit and an edge from each commit to
// each of its (visible) parents using the precomputed columns.

export type GraphInput = { sha: string; parents: string[] };

export type GraphNode = {
  sha: string;
  column: number;
  color: number;
};

export type GraphLayout = {
  /** Per-sha node (column + color). */
  nodes: Map<string, GraphNode>;
  /** Same nodes in input order. */
  order: GraphNode[];
  /** Total number of columns used (for SVG width). */
  columns: number;
};

type Lane = { sha: string; color: number } | null;

export function computeGraph(commits: GraphInput[]): GraphLayout {
  const lanes: Lane[] = [];
  const nodes = new Map<string, GraphNode>();
  const order: GraphNode[] = [];
  let nextColor = 0;
  let maxColumn = 0;

  const firstFreeLane = (): number => {
    const idx = lanes.indexOf(null);
    return idx === -1 ? lanes.length : idx;
  };

  for (const commit of commits) {
    // Find the lane that was waiting for this commit (a child reserved it).
    let column = lanes.findIndex((lane) => lane?.sha === commit.sha);
    let color: number;
    if (column === -1) {
      column = firstFreeLane();
      color = nextColor++;
      lanes[column] = { sha: commit.sha, color };
    } else {
      color = lanes[column]!.color;
    }

    // Collapse any other lanes also waiting for this commit (merge target).
    for (let i = 0; i < lanes.length; i += 1) {
      if (i !== column && lanes[i]?.sha === commit.sha) {
        lanes[i] = null;
      }
    }

    const node: GraphNode = { sha: commit.sha, column, color };
    nodes.set(commit.sha, node);
    order.push(node);
    maxColumn = Math.max(maxColumn, column);

    // Route parents: first parent continues this lane; extra parents open lanes.
    commit.parents.forEach((parent, index) => {
      if (index === 0) {
        lanes[column] = { sha: parent, color };
      } else if (!lanes.some((lane) => lane?.sha === parent)) {
        const lane = firstFreeLane();
        lanes[lane] = { sha: parent, color: nextColor++ };
        maxColumn = Math.max(maxColumn, lane);
      }
    });
    if (commit.parents.length === 0) {
      lanes[column] = null;
    }

    // Trim trailing empty lanes so width reflects active columns.
    while (lanes.length > 0 && lanes[lanes.length - 1] === null) {
      lanes.pop();
    }
    maxColumn = Math.max(maxColumn, lanes.length - 1);
  }

  return { nodes, order, columns: maxColumn + 1 };
}
