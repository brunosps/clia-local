import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";

/** Rendered markdown preview (GitHub-flavored, no raw HTML). */
export function MarkdownPreview({ text }: { text: string }) {
  return (
    <div className="markdown-preview">
      <Markdown remarkPlugins={[remarkGfm]}>{text}</Markdown>
    </div>
  );
}
