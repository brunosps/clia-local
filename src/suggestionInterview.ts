import { extractJsonObject } from "./interview";

// Agent-driven suggestion interview. The agent runs the flow's suggest command
// (e.g. /dw-brainstorm) and emits ITS clarifying questions one at a time as JSON;
// when it has enough context it finalizes with a LIST of candidate features /
// refactors, which the GUI turns into backlog (intake) cards.

export type SuggestionInterviewTurn = { question: string; answer: string };

export type SuggestionKind = "feature" | "refactor";

export type SuggestionItem = { title: string; body: string; kind?: SuggestionKind };

export type SuggestionInterviewResponse =
  | { state: "question"; question: string; options?: string[] }
  | { state: "working"; message?: string }
  | { state: "done"; suggestions: SuggestionItem[] };

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function parseSuggestionItem(value: unknown): SuggestionItem | null {
  if (!isRecord(value)) return null;
  const title = typeof value.title === "string" ? value.title.trim() : "";
  if (!title) return null;
  const body = typeof value.body === "string" ? value.body.trim() : "";
  const kind = value.kind === "feature" || value.kind === "refactor" ? value.kind : undefined;
  return { title, body, kind };
}

/** Parse an agent message into the next suggestion-interview step. */
export function parseSuggestionInterviewResponse(content: string): SuggestionInterviewResponse | null {
  const rawJson = extractJsonObject(content);
  if (!rawJson) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(rawJson);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;

  if (parsed.state === "done") {
    const suggestions = Array.isArray(parsed.suggestions)
      ? parsed.suggestions.map(parseSuggestionItem).filter((item): item is SuggestionItem => item !== null)
      : [];
    return { state: "done", suggestions };
  }
  if (parsed.state === "working") {
    return { state: "working", message: typeof parsed.message === "string" ? parsed.message : undefined };
  }
  if (parsed.state === "question") {
    const question = typeof parsed.question === "string" ? parsed.question.trim() : "";
    if (!question) return null;
    const options = Array.isArray(parsed.options)
      ? parsed.options.filter((option): option is string => typeof option === "string" && option.trim() !== "")
      : undefined;
    return { state: "question", question, options: options && options.length ? options : undefined };
  }
  return null;
}

/** Initial prompt: run the flow's suggest command as an interview. */
export function suggestionInterviewPrompt(args: { projectName: string; suggestCommand: string }): string {
  return [
    `Você vai levantar oportunidades para o projeto "${args.projectName}" executando o comando ${args.suggestCommand}.`,
    "Considere oportunidades de produto, UX, automação, alavancagem técnica, REFATORAÇÃO e segurança — não só features.",
    "O objetivo é virarem itens de backlog. Conduza como uma ENTREVISTA: faça UMA pergunta por vez para entender",
    "prioridades, foco e contexto antes de propor.",
    "",
    "Responda SOMENTE com JSON válido, sem markdown e sem texto fora do JSON. Use exatamente um destes estados:",
    '- Para perguntar (precisa de resposta): {"state":"question","question":"...","options":["sugestão curta","..."]}',
    "    options é OPCIONAL: inclua sugestões quando ajudar; omita para pergunta aberta. Uma pergunta por vez.",
    '- Para progresso (sem precisar de resposta): {"state":"working","message":"o que está fazendo agora"}',
    "- Quando tiver contexto suficiente, finalize com a LISTA de oportunidades (NÃO escreva arquivos):",
    '    {"state":"done","suggestions":[{"title":"título curto","body":"1-3 frases de descrição/justificativa","kind":"feature"|"refactor"}, ...]}',
    "",
    "Regras da lista final: de 3 a 7 itens concisos, cada um acionável; title curto; body explica o valor;",
    'kind é OPCIONAL ("feature" ou "refactor" quando se aplicar). Só emita {"state":"done"} quando a lista estiver pronta.',
  ].join("\n");
}

/** Follow-up prompt carrying the user's answer to the current question. */
export function suggestionAnswerPrompt(answer: string): string {
  return [
    `Resposta do usuário: ${answer}`,
    "",
    "Continue de onde parou. Próxima etapa em JSON:",
    '- outra pergunta {"state":"question",...} se precisar de mais contexto,',
    '- progresso {"state":"working",...} enquanto pensa,',
    '- {"state":"done","suggestions":[...]} quando a lista de ideias estiver pronta.',
  ].join("\n");
}
