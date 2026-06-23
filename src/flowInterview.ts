import { extractJsonObject, parseQuestion, type InterviewOptionKey, type InterviewQuestion } from "./interview";
import { PRESET_SPEC_KIT } from "./flows";

export const FLOW_INTERVIEW_MAX_QUESTIONS = 6;

export type FlowInterviewTurn = {
  question: string;
  selected: InterviewOptionKey;
  answer: string;
  note?: string;
};

export type FlowInterviewResponse =
  | ({ state: "question" } & InterviewQuestion)
  | { state: "final"; flow: string; id?: string; label?: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

/** Parse an agent message into the next flow-interview step (question or final flow). */
export function parseFlowInterviewResponse(content: string): FlowInterviewResponse | null {
  const rawJson = extractJsonObject(content);
  if (!rawJson) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(rawJson);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;

  if (parsed.state === "question") {
    const question = parseQuestion(parsed);
    return question ? { state: "question", ...question } : null;
  }
  if (parsed.state === "final") {
    const flow =
      typeof parsed.flow === "string"
        ? parsed.flow
        : isRecord(parsed.flow)
          ? JSON.stringify(parsed.flow, null, 2)
          : "";
    if (!flow.trim()) return null;
    return {
      state: "final",
      flow,
      id: typeof parsed.id === "string" ? parsed.id : undefined,
      label: typeof parsed.label === "string" ? parsed.label : undefined,
    };
  }
  return null;
}

const SCHEMA_SPEC = [
  "Formato do flow (WorkbenchSchema):",
  '- Objeto: { "version": 1, "newCard"?: { "fields": Field[] }, "groups"?: Group[], "phases": Phase[] }.',
  '- Phase: { "id": slug-único, "label", "status": snake_case-único, "description", "fields": Field[], "action": Action, "advance"?: Advance, "group"?: id-de-group, "wipLimit"?: number }.',
  '- Action (um destes): { "type":"command", "base":"/comando", "promptParts"?:[{ "template":"texto com {{card.title}} ou {{field.key}}" }] } | { "type":"interview", "style":"horizons", "fillsFields":[...] } | { "type":"skill", "skill":"$skill" } | { "type":"none" }.',
  '- Field: { "key", "label", "type": "text|textarea|select|multiselect|checklist|projects|attachments", "required"?, "placeholder"?, "options"?:[{ "value","label" }], "binding"?: "title|body" }.',
  '- Group: { "id", "label" }. Advance: { "requireFields"?:[keys], "expectJson"?:[keys] }.',
  "Os comandos (base) devem ser os comandos/slash reais da ferramenta documentada.",
].join("\n");

/**
 * Build the prompt that drives the URL→flow interview. The agent asks a few
 * H1-H4 clarifying questions then emits the final flow JSON. `pageContent` is
 * the fetched docs (when available); otherwise the agent is told to fetch it.
 */
export function flowInterviewPrompt(args: {
  url: string;
  pageContent?: string;
  turns: FlowInterviewTurn[];
}): string {
  const { url, pageContent, turns } = args;
  const lines: string[] = [
    `Você ajuda a montar um fluxo de trabalho (workbench flow) no formato abaixo para a ferramenta documentada em: ${url}`,
    "",
    pageContent
      ? `Conteúdo da página (truncado):\n\`\`\`\n${pageContent}\n\`\`\``
      : `Não foi possível baixar a página. Use sua ferramenta de WebFetch para ler ${url} antes de propor o fluxo.`,
    "",
    SCHEMA_SPEC,
    "",
    "Exemplo de um flow válido (spec-kit):",
    "```json",
    JSON.stringify(PRESET_SPEC_KIT),
    "```",
    "",
    "Responda SOMENTE com JSON válido, sem markdown e sem texto fora do JSON.",
    "Regras:",
    `- Faça no máximo ${FLOW_INTERVIEW_MAX_QUESTIONS} perguntas para entender as fases que o usuário quer, cada uma com quatro opções de horizonte: H1, H2, H3, H4.`,
    "- H1: conservador, fiel ao pipeline padrão da ferramenta. H2: mais ousado. H3: disruptivo. H4: contraponto.",
    "- Quando tiver informação suficiente, emita o estado final com o flow completo e correto para a ferramenta.",
    "",
    "Formato para pergunta:",
    JSON.stringify({
      state: "question",
      question_number: 1,
      question: "...",
      options: { H1: "...", H2: "...", H3: "...", H4: "..." },
      running_summary: "...",
    }),
    "",
    "Formato final (id = slug curto, label = nome legível, flow = objeto WorkbenchSchema):",
    JSON.stringify({ state: "final", id: "minha-ferramenta", label: "Minha ferramenta", flow: { version: 1, phases: ["..."] } }),
  ];

  if (turns.length > 0) {
    lines.push("", "Respostas anteriores do usuário:");
    turns.forEach((turn, index) => {
      lines.push(
        `${index + 1}. Pergunta: ${turn.question}`,
        `   Escolha (${turn.selected}): ${turn.answer}${turn.note ? ` — nota: ${turn.note}` : ""}`,
      );
    });
    lines.push("", "Continue a entrevista (próxima pergunta) ou emita o estado final.");
  }

  return lines.join("\n");
}
