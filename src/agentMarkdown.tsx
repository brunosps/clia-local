import { isValidElement, memo, useState, type ComponentProps, type ReactNode } from "react";
import { Check, Copy } from "lucide-react";
import Markdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/atom-one-dark.css";
import { useI18n } from "./i18n";
import type { AgentMessage } from "./types";

// highlight.js: only colorize blocks with an explicit language tag (```sql, ```ts, …);
// don't throw on unknown tags (e.g. advpl). `detect` is deliberately OFF — auto-detecting
// the language runs every bundled grammar against every block on each render, which pegs
// the UI thread while streaming a long agent reply (re-highlighting all messages per token)
// and froze the webview on WSLg. highlight.js ships no AdvPL grammar anyway, so detection
// only ever mis-tagged AdvPL/TLPP — dropping it costs nothing and keeps the tab responsive.
const rehypePlugins: ComponentProps<typeof Markdown>["rehypePlugins"] = [
  [rehypeHighlight, { ignoreMissing: true }],
];

type FileLinkProps = {
  /** Resolve a token (e.g. an inline-code file reference) to a real project-relative path. */
  resolveSourceRef?: (token: string) => string | null;
  /** Open a project-relative file in the Code tab. */
  onOpenSourceFile?: (relativePath: string) => void;
};

/** Flatten a React node tree to its raw text (used to copy fenced code blocks). */
function nodeToText(node: ReactNode): string {
  if (node == null || typeof node === "boolean") return "";
  if (typeof node === "string") return node;
  if (typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(nodeToText).join("");
  if (isValidElement(node)) {
    return nodeToText((node.props as { children?: ReactNode }).children);
  }
  return "";
}

/** A fenced code block with a copy-to-clipboard button in the top-right corner. */
function CodeBlock({ children }: { children?: ReactNode }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);
  const code = nodeToText(children).replace(/\n+$/, "");
  const copy = () => {
    if (!navigator.clipboard) return;
    void navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    });
  };
  return (
    <div className="agent-code-block">
      <button
        type="button"
        className="agent-code-copy"
        onClick={copy}
        aria-label={t("agents.copyCode")}
        title={copied ? t("agents.codeCopied") : t("agents.copyCode")}
      >
        {copied ? <Check aria-hidden="true" size={13} /> : <Copy aria-hidden="true" size={13} />}
      </button>
      <pre>{children}</pre>
    </div>
  );
}

function buildComponents({ resolveSourceRef, onOpenSourceFile }: FileLinkProps): Components {
  const fileLink: Partial<Components> =
    resolveSourceRef && onOpenSourceFile
      ? {
          code({ className, children }) {
            // Inline code (no language-* class) that names a real project file → link.
            const text = String(children ?? "");
            const relativePath = className ? null : resolveSourceRef(text);
            if (relativePath) {
              return (
                <button
                  type="button"
                  className="agent-file-link"
                  title={relativePath}
                  onClick={() => onOpenSourceFile(relativePath)}
                >
                  {children}
                </button>
              );
            }
            return <code className={className}>{children}</code>;
          },
        }
      : {};
  return {
    pre({ children }) {
      return <CodeBlock>{children}</CodeBlock>;
    },
    ...fileLink,
  };
}

function shouldRenderAgentMarkdown(message: Pick<AgentMessage, "role" | "content">) {
  return message.role === "assistant" && message.content.trim().length > 0;
}

/** Render arbitrary text as GFM markdown (used by the task run-history prompt). */
export function AgentMarkdown({ text, ...links }: { text: string } & FileLinkProps) {
  return (
    <div className="agent-message-markdown">
      <Markdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={rehypePlugins}
        components={buildComponents(links)}
      >
        {text}
      </Markdown>
    </div>
  );
}

// Memoized: the timeline maps every message through this, so without memoization a single
// streaming token (which replaces only the in-flight message) would re-render — and
// re-highlight — every prior message. `appendAgentStreamingDelta` keeps stable references
// for untouched messages and the link callbacks are `useCallback`-stable, so the default
// shallow comparison correctly re-renders only the message whose content actually changed.
export const AgentMessageContent = memo(function AgentMessageContent({
  message,
  ...links
}: { message: AgentMessage } & FileLinkProps) {
  if (message.content) {
    if (shouldRenderAgentMarkdown(message)) {
      return (
        <div className="agent-message-markdown">
          <Markdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={rehypePlugins}
        components={buildComponents(links)}
      >
            {message.content}
          </Markdown>
        </div>
      );
    }

    return <p className="agent-message-text">{message.content}</p>;
  }

  if (message.raw_json) {
    return <code className="agent-message-raw">{message.raw_json}</code>;
  }

  return null;
});
