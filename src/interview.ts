export const INTERVIEW_MIN_QUESTIONS = 5;
export const INTERVIEW_MAX_QUESTIONS = 9;

export type InterviewOptionKey = "H1" | "H2" | "H3" | "H4";

export type InterviewQuestion = {
  question_number: number;
  question: string;
  options: Record<InterviewOptionKey, string>;
  running_summary?: string;
};

export type InterviewDraft = {
  description: string;
  context: string;
  expected_result: string;
  checklist: string[];
  summary: string;
};

export type InterviewAgentResponse =
  | ({ state: "question" } & InterviewQuestion)
  | ({ state: "final" } & InterviewDraft);

export type InterviewTurn = {
  question_number: number;
  question: string;
  selected: InterviewOptionKey;
  answer: string;
  note?: string;
};

export type BacklogInterviewState = {
  status?: "idle" | "asking" | "ready" | "applied" | "failed";
  agent_profile_id?: number;
  session_id?: number;
  current?: InterviewQuestion;
  draft?: InterviewDraft;
  turns?: InterviewTurn[];
  summary?: string;
  error?: string;
  updated_at?: string;
};

export function parseInterviewAgentResponse(content: string): InterviewAgentResponse | null {
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
    const draft = parseDraft(parsed);
    return draft ? { state: "final", ...draft } : null;
  }
  return null;
}

export function interviewStateFromValue(value: unknown): BacklogInterviewState {
  if (!isRecord(value)) return { status: "idle", turns: [] };
  const state: BacklogInterviewState = {
    status: isInterviewStatus(value.status) ? value.status : "idle",
    agent_profile_id: typeof value.agent_profile_id === "number" ? value.agent_profile_id : undefined,
    session_id: typeof value.session_id === "number" ? value.session_id : undefined,
    turns: parseTurns(value.turns),
    summary: typeof value.summary === "string" ? value.summary : undefined,
    error: typeof value.error === "string" ? value.error : undefined,
    updated_at: typeof value.updated_at === "string" ? value.updated_at : undefined,
  };
  if (isRecord(value.current)) {
    state.current = parseQuestion(value.current);
  }
  if (isRecord(value.draft)) {
    state.draft = parseDraft(value.draft);
  }
  return state;
}

export function parseQuestion(value: Record<string, unknown>): InterviewQuestion | undefined {
  const options = value.options;
  if (!isRecord(options)) return undefined;
  const h1 = text(options.H1);
  const h2 = text(options.H2);
  const h3 = text(options.H3);
  const h4 = text(options.H4);
  const question = text(value.question);
  const questionNumber = Number(value.question_number);
  if (!question || !h1 || !h2 || !h3 || !h4 || !Number.isFinite(questionNumber)) {
    return undefined;
  }
  return {
    question_number: questionNumber,
    question,
    options: { H1: h1, H2: h2, H3: h3, H4: h4 },
    running_summary: text(value.running_summary),
  };
}

function parseDraft(value: Record<string, unknown>): InterviewDraft | undefined {
  const description = text(value.description);
  const context = text(value.context);
  const expectedResult = text(value.expected_result);
  const summary = text(value.summary);
  const checklist = Array.isArray(value.checklist)
    ? value.checklist.filter(
        (item): item is string => typeof item === "string" && Boolean(item.trim()),
      )
    : [];
  if (!description || !context || !expectedResult || !summary || !checklist.length) {
    return undefined;
  }
  return { description, context, expected_result: expectedResult, checklist, summary };
}

function parseTurns(value: unknown): InterviewTurn[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter((item): item is Record<string, unknown> => isRecord(item))
    .map((item) => ({
      question_number: Number(item.question_number) || 0,
      question: text(item.question),
      selected: isOptionKey(item.selected) ? item.selected : "H1",
      answer: text(item.answer),
      note: text(item.note),
    }))
    .filter((item) => item.question && item.answer);
}

export function extractJsonObject(content: string) {
  const fenced = content.match(/```(?:json)?\s*([\s\S]*?)```/i);
  if (fenced?.[1]) return fenced[1].trim();
  const start = content.indexOf("{");
  const end = content.lastIndexOf("}");
  if (start === -1 || end <= start) return "";
  return content.slice(start, end + 1).trim();
}

function text(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isOptionKey(value: unknown): value is InterviewOptionKey {
  return value === "H1" || value === "H2" || value === "H3" || value === "H4";
}

function isInterviewStatus(value: unknown): value is BacklogInterviewState["status"] {
  return (
    value === "idle" ||
    value === "asking" ||
    value === "ready" ||
    value === "applied" ||
    value === "failed"
  );
}
