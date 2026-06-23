import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { AgentMessage } from "./types";

function shouldRenderAgentMarkdown(message: Pick<AgentMessage, "role" | "content">) {
  return message.role === "assistant" && message.content.trim().length > 0;
}

export function AgentMessageContent({ message }: { message: AgentMessage }) {
  if (message.content) {
    if (shouldRenderAgentMarkdown(message)) {
      return (
        <div className="agent-message-markdown">
          <Markdown remarkPlugins={[remarkGfm]}>{message.content}</Markdown>
        </div>
      );
    }

    return <p className="agent-message-text">{message.content}</p>;
  }

  if (message.raw_json) {
    return <code className="agent-message-raw">{message.raw_json}</code>;
  }

  return null;
}
