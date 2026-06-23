// Helpers for the "AI Commit" button: build a prompt from the staged diff and
// clean the agent's reply down to a bare commit message for the textarea.

const MAX_DIFF_CHARS = 12000;

export type CommitPromptFile = {
  path: string;
  status: string;
  additions?: number;
  deletions?: number;
};

type CommitPromptOptions = {
  projectName?: string;
  stagedFiles?: CommitPromptFile[];
};

/** Prompt the agent for ONE Conventional Commits message from the staged diff. */
export function buildCommitMessagePrompt(diff: string, options: CommitPromptOptions = {}): string {
  const trimmed =
    diff.length > MAX_DIFF_CHARS ? `${diff.slice(0, MAX_DIFF_CHARS)}\n…(diff truncado)` : diff;
  const files = options.stagedFiles?.length
    ? options.stagedFiles
        .map((file) => {
          const stats =
            file.additions != null || file.deletions != null
              ? ` +${file.additions ?? 0} -${file.deletions ?? 0}`
              : "";
          return `${file.status.padEnd(2)} ${file.path}${stats}`;
        })
        .join("\n")
    : "(use o --stat do diff staged abaixo)";
  return [
    "Você é o gerador de mensagem do botão AI Commit, inspirado no comando /dw-commit.",
    "Atue como especialista em Git e versionamento. Analise SOMENTE as mudanças STAGED abaixo e siga o padrão Conventional Commits.",
    "Não execute git commit, git add, git restore, git diff nem qualquer outro comando; apenas gere a mensagem final.",
    options.projectName ? `Projeto: ${options.projectName}` : null,
    "",
    "Fluxo obrigatório:",
    "1. Leia a lista de arquivos, o --stat e o patch staged.",
    "2. Identifique a intenção lógica dominante e o módulo/área afetado.",
    "3. Escolha um type válido: feat, fix, docs, style, refactor, perf, test, chore, ci ou build.",
    "4. Escolha um scope curto e específico (ex: git, agents, workbench, rules, tauri).",
    "5. Escreva um subject específico, no formato `type(scope): resumo`, com no máximo 72 caracteres.",
    "6. Se o staged diff tocar vários arquivos, criar configs/regras, ou passar de ~200 linhas, inclua uma descrição curta após uma linha em branco.",
    "",
    "Critérios de qualidade:",
    "- Explique O QUE mudou e, na descrição, o impacto prático quando houver contexto.",
    "- Prefira commits atômicos: uma intenção lógica, sem misturar assuntos no subject.",
    "- Não use mensagens vagas como `updates`, `fix stuff`, `mudanças`, `arquivos`, `regras e fluxos` sem dizer quais/para quê.",
    "- Não invente nada que não esteja no staged diff.",
    "- Não mencione arquivos unstaged.",
    "",
    "Formato de resposta:",
    "Responda SOMENTE com a mensagem de commit, sem markdown, sem aspas, sem explicação e sem rótulos `Subject:`/`Description:`.",
    "Para diffs não triviais, use:",
    "`type(scope): resumo específico`",
    "",
    "Corpo curto em 1-3 linhas explicando as principais mudanças e o impacto.",
    "",
    "Arquivos staged:",
    files,
    "",
    "Diff staged:",
    "```diff",
    trimmed.trim() ? trimmed : "(sem mudanças detectadas)",
    "```",
  ]
    .filter((line): line is string => line != null)
    .join("\n");
}

/** Strip code fences / surrounding quotes the model may add around the message. */
export function cleanCommitMessage(content: string): string {
  let text = content.trim();
  // Drop a leading/trailing fenced block, keeping its inner content.
  const fence = text.match(/^```[^\n]*\n([\s\S]*?)\n?```$/);
  if (fence) text = fence[1].trim();
  // Strip a single layer of wrapping quotes/backticks.
  text = text.replace(/^(["'`])([\s\S]*)\1$/, "$2").trim();
  text = text
    .replace(/^Subject:\s*/i, "")
    .replace(/\n\s*Description:\s*/i, "\n\n")
    .trim();
  return text;
}

export function latestAssistantCommitMessage(
  messages: Array<{ role: string; content: string }>,
): string | null {
  const message = [...messages]
    .reverse()
    .find((item) => item.role === "assistant" && item.content.trim());
  return message ? cleanCommitMessage(message.content) : null;
}

export function latestSystemMessage(
  messages: Array<{ role: string; content: string }>,
): string | null {
  return (
    [...messages]
      .reverse()
      .find((item) => item.role === "system" && item.content.trim())
      ?.content.trim() ?? null
  );
}

export function splitCommitMessage(message: string): { subject: string; description: string } {
  const normalized = message.replace(/\r\n/g, "\n").trim();
  if (!normalized) return { subject: "", description: "" };
  const [subject = "", ...descriptionLines] = normalized.split("\n");
  return {
    subject: subject.trim(),
    description: descriptionLines.join("\n").trim(),
  };
}

export function composeCommitMessage(subject: string, description: string): string {
  const cleanSubject = subject.trim();
  const cleanDescription = description.trim();
  if (!cleanSubject) return cleanDescription;
  if (!cleanDescription) return cleanSubject;
  return `${cleanSubject}\n\n${cleanDescription}`;
}

export function aiCommitProfileKey(projectId: number): string {
  return `ai_commit_profile:${projectId}`;
}

export function resolveAiCommitProfileId(
  profiles: Array<{ id: number }>,
  preferredId: number | null,
  activeId: number | null,
): number | null {
  if (preferredId != null && profiles.some((profile) => profile.id === preferredId)) {
    return preferredId;
  }
  if (activeId != null && profiles.some((profile) => profile.id === activeId)) {
    return activeId;
  }
  return profiles[0]?.id ?? null;
}
