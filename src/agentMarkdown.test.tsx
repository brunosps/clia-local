import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AgentMessageContent } from "./agentMarkdown";
import type { AgentMessage } from "./types";

function message(overrides: Partial<AgentMessage>): AgentMessage {
  return {
    id: 1,
    session_id: 1,
    role: "assistant",
    content: "",
    raw_json: null,
    created_at: "2026-05-29T00:00:00.000Z",
    ...overrides,
  };
}

describe("agent markdown rendering", () => {
  it("renders assistant markdown as structured HTML", () => {
    const html = renderToStaticMarkup(
      <AgentMessageContent
        message={message({
          content: "**winbox-gui**\n\n- cria VMs\n- usa `Docker`",
        })}
      />,
    );

    expect(html).toContain("<strong>winbox-gui</strong>");
    expect(html).toContain("<ul>");
    expect(html).toContain("<li>cria VMs</li>");
    expect(html).toContain("<code>Docker</code>");
    expect(html).not.toContain("**winbox-gui**");
  });

  it("keeps user content literal instead of rendering markdown", () => {
    const html = renderToStaticMarkup(
      <AgentMessageContent message={message({ role: "user", content: "**literal**" })} />,
    );

    expect(html).toContain("**literal**");
    expect(html).not.toContain("<strong>");
  });

  it("escapes raw html from assistant messages", () => {
    const html = renderToStaticMarkup(
      <AgentMessageContent message={message({ content: "<script>alert(1)</script>" })} />,
    );

    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;alert(1)&lt;/script&gt;");
  });

  it("renders blank assistant messages as empty content", () => {
    const html = renderToStaticMarkup(<AgentMessageContent message={message({ content: "   " })} />);

    expect(html).toContain("   ");
    expect(html).not.toContain("agent-message-markdown");
  });
});
