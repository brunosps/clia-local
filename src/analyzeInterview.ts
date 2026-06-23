import { extractJsonObject } from "./interview";

// Agent-driven project analysis interview. The agent runs the flow's analyze
// command (e.g. /dw-analyze-project) and emits ITS clarifying questions one at a
// time as JSON; when it has enough context it finalizes and writes the rules.

export type AnalyzeInterviewTurn = { question: string; answer: string };

export type AnalyzeInterviewResponse =
  | { state: "question"; question: string; options?: string[] }
  | { state: "working"; message?: string }
  | { state: "done" };

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

/** Parse an agent message into the next analyze-interview step (question or final). */
export function parseAnalyzeInterviewResponse(content: string): AnalyzeInterviewResponse | null {
  const rawJson = extractJsonObject(content);
  if (!rawJson) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(rawJson);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;

  // The agent signals true completion with "done"; tolerate the older "final".
  if (parsed.state === "done" || parsed.state === "final") return { state: "done" };
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

/** Initial prompt: run the flow's analyze command end-to-end in interview mode. */
export function analyzeInterviewPrompt(args: { projectName: string; analyzeCommand: string }): string {
  return [
    `Você vai analisar o projeto "${args.projectName}" executando o comando ${args.analyzeCommand} INTEIRO, do início ao fim.`,
    "Conduza TODAS as etapas do comando como uma ENTREVISTA: as perguntas de esclarecimento, a geração das",
    "rules/contexto e QUALQUER etapa adicional (ex.: constitution, validações). Faça UMA pergunta por vez.",
    "",
    "Responda SOMENTE com JSON válido, sem markdown e sem texto fora do JSON. Use exatamente um destes estados:",
    '- Para perguntar (precisa de resposta): {"state":"question","question":"...","options":["sugestão curta","..."]}',
    "    options é OPCIONAL: inclua sugestões quando ajudar; omita para pergunta aberta. Uma pergunta por vez.",
    '- Para progresso (sem precisar de resposta): {"state":"working","message":"o que está fazendo agora"}',
    '- SÓ quando o comando estiver 100% concluído e TODOS os arquivos escritos: {"state":"done"}',
    "",
    'IMPORTANTE: não emita {"state":"done"} antes de terminar tudo. Se ainda faltar uma etapa (mesmo opcional,',
    "como constitution), trate-a: pergunte com \"question\" se precisar de decisão, ou execute e reporte com \"working\".",
    'O "done" é o ÚNICO sinal de término — só o envie quando não houver mais nada a fazer no comando.',
  ].join("\n");
}

/** Follow-up prompt carrying the user's answer to the current question. */
export function analyzeAnswerPrompt(answer: string): string {
  return [
    `Resposta do usuário: ${answer}`,
    "",
    "Continue o comando de onde parou. Próxima etapa em JSON:",
    '- outra pergunta {"state":"question",...} se precisar de mais alguma decisão,',
    '- progresso {"state":"working",...} enquanto executa/escreve,',
    '- {"state":"done"} APENAS quando o comando inteiro estiver concluído e todos os arquivos escritos.',
  ].join("\n");
}
