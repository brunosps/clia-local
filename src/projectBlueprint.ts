import { extractJsonObject } from "./interview";
import type { KnowledgeSource } from "./types";

export type ProjectBlueprintQuestion = {
  id: string;
  area: string;
  question: string;
};

export type ProjectBlueprintAnswer = ProjectBlueprintQuestion & {
  answer: string;
};

export type ProjectBlueprintTask = {
  title: string;
  body: string;
  dependencies?: string[];
};

export type ProjectBlueprintAgentResponse =
  | {
      state: "question_batch";
      questions: ProjectBlueprintQuestion[];
      running_summary?: string;
      detected_subprojects?: string[];
    }
  | {
      state: "final_plan";
      running_summary: string;
      detected_subprojects: string[];
      prd: string;
      techspec: string;
      tasks: ProjectBlueprintTask[];
      definition_of_done: string;
    };

export const PROJECT_BLUEPRINT_BATCH_SIZE = 3;

export const PROJECT_BLUEPRINT_QUESTION_BANK: ProjectBlueprintQuestion[] = [
  { id: "business-01", area: "Negócio", question: "Qual problema real este projeto precisa resolver?" },
  { id: "business-02", area: "Negócio", question: "Quem sente esse problema hoje e como ele aparece no processo atual?" },
  { id: "business-03", area: "Negócio", question: "Qual resultado mensurável define que o projeto valeu a pena?" },
  { id: "business-04", area: "Negócio", question: "Existe prazo, janela comercial ou compromisso externo?" },
  { id: "business-05", area: "Negócio", question: "Quais decisões de negócio ainda estão abertas?" },
  { id: "business-06", area: "Negócio", question: "Que restrições de orçamento, equipe ou operação precisam entrar no plano?" },
  { id: "business-07", area: "Negócio", question: "Quais riscos de negócio tornam esse projeto inviável se ignorados?" },
  { id: "product-01", area: "Produto", question: "Quem é o usuário primário e qual tarefa ele quer completar?" },
  { id: "product-02", area: "Produto", question: "Quais personas secundárias precisam ser consideradas?" },
  { id: "product-03", area: "Produto", question: "Qual fluxo principal deve existir na primeira versão?" },
  { id: "product-04", area: "Produto", question: "O que explicitamente não entra no MVP?" },
  { id: "product-05", area: "Produto", question: "Quais telas, jornadas ou comandos o usuário espera encontrar?" },
  { id: "product-06", area: "Produto", question: "Quais estados vazios, erros e confirmações são importantes?" },
  { id: "product-07", area: "Produto", question: "Como o usuário percebe progresso e sucesso dentro do produto?" },
  { id: "scope-01", area: "Escopo", question: "Isso é um projeto único ou pode virar múltiplos projetos/repos?" },
  { id: "scope-02", area: "Escopo", question: "Quais módulos ou subprodutos você imagina que podem nascer daqui?" },
  { id: "scope-03", area: "Escopo", question: "Quais dependências entre subprojetos precisam ser respeitadas?" },
  { id: "scope-04", area: "Escopo", question: "Existe algo que deve ser entregue como biblioteca, serviço, app ou CLI separado?" },
  { id: "scope-05", area: "Escopo", question: "Quais partes podem ser adiadas sem quebrar o valor central?" },
  { id: "scope-06", area: "Escopo", question: "O projeto precisa conversar com produtos já existentes no workspace?" },
  { id: "scope-07", area: "Escopo", question: "Que nome ou estrutura de pastas faria sentido para o novo projeto?" },
  { id: "functional-01", area: "Requisitos funcionais", question: "Quais ações o usuário precisa executar no sistema?" },
  { id: "functional-02", area: "Requisitos funcionais", question: "Quais entidades, cadastros ou recursos precisam existir?" },
  { id: "functional-03", area: "Requisitos funcionais", question: "Quais permissões, papéis ou aprovações entram no fluxo?" },
  { id: "functional-04", area: "Requisitos funcionais", question: "Quais automações o sistema deve executar sozinho?" },
  { id: "functional-05", area: "Requisitos funcionais", question: "Quais notificações, logs ou auditorias são obrigatórios?" },
  { id: "functional-06", area: "Requisitos funcionais", question: "Quais relatórios, buscas ou filtros precisam existir?" },
  { id: "functional-07", area: "Requisitos funcionais", question: "Quais integrações externas entram no comportamento funcional?" },
  { id: "nonfunctional-01", area: "Requisitos não funcionais", question: "Há metas de performance, latência ou volume?" },
  { id: "nonfunctional-02", area: "Requisitos não funcionais", question: "Há requisitos de disponibilidade ou recuperação de falha?" },
  { id: "nonfunctional-03", area: "Requisitos não funcionais", question: "Quais dados são sensíveis e como precisam ser protegidos?" },
  { id: "nonfunctional-04", area: "Requisitos não funcionais", question: "Há requisitos legais, LGPD, auditoria ou compliance?" },
  { id: "nonfunctional-05", area: "Requisitos não funcionais", question: "O sistema precisa funcionar offline, em rede ruim ou em VM?" },
  { id: "nonfunctional-06", area: "Requisitos não funcionais", question: "Que observabilidade é necessária para operar isso?" },
  { id: "nonfunctional-07", area: "Requisitos não funcionais", question: "Quais requisitos de acessibilidade e usabilidade são obrigatórios?" },
  { id: "stack-01", area: "Stack", question: "Existe stack obrigatória ou preferida?" },
  { id: "stack-02", area: "Stack", question: "Quais linguagens, frameworks e runtimes devem ser evitados?" },
  { id: "stack-03", area: "Stack", question: "Quais bancos, filas, caches ou storage são esperados?" },
  { id: "stack-04", area: "Stack", question: "O projeto precisa de frontend, backend, desktop, mobile, CLI ou API?" },
  { id: "stack-05", area: "Stack", question: "Quais ambientes precisam rodar: local, Docker, VM, staging, produção?" },
  { id: "stack-06", area: "Stack", question: "Que ferramentas de teste, build e CI precisam entrar desde o início?" },
  { id: "stack-07", area: "Stack", question: "Há dependências de serviços terceiros, SDKs ou APIs específicas?" },
  { id: "data-01", area: "Dados", question: "Quais dados entram, saem e ficam persistidos?" },
  { id: "data-02", area: "Dados", question: "Qual é a origem dos dados iniciais?" },
  { id: "data-03", area: "Dados", question: "Há importação, exportação ou sincronização?" },
  { id: "data-04", area: "Dados", question: "Quais campos precisam de histórico, versionamento ou auditoria?" },
  { id: "data-05", area: "Dados", question: "Quais regras de retenção, anonimização ou exclusão existem?" },
  { id: "data-06", area: "Dados", question: "Como erros de dados devem ser tratados e explicados?" },
  { id: "data-07", area: "Dados", question: "Que contratos de API ou eventos precisam ser definidos?" },
  { id: "devops-01", area: "DevOps", question: "Como o projeto deve ser instalado em ambiente de desenvolvimento?" },
  { id: "devops-02", area: "DevOps", question: "Como deve funcionar o deploy local, em VM ou em produção?" },
  { id: "devops-03", area: "DevOps", question: "Quais secrets e variáveis de ambiente existem?" },
  { id: "devops-04", area: "DevOps", question: "O projeto precisa de Dockerfile, Compose ou scripts de bootstrap?" },
  { id: "devops-05", area: "DevOps", question: "Quais comandos devem existir para build, test, dev e release?" },
  { id: "devops-06", area: "DevOps", question: "Que validações automatizadas bloqueiam entrega?" },
  { id: "devops-07", area: "DevOps", question: "Como rollback, limpeza e troubleshooting devem funcionar?" },
  { id: "patterns-01", area: "Patterns", question: "Quais padrões arquiteturais combinam com o domínio?" },
  { id: "patterns-02", area: "Patterns", question: "Qual fronteira de módulos reduz acoplamento?" },
  { id: "patterns-03", area: "Patterns", question: "Onde faz sentido aplicar filas, eventos ou jobs assíncronos?" },
  { id: "patterns-04", area: "Patterns", question: "Onde a UI precisa ser guiada, densa ou exploratória?" },
  { id: "patterns-05", area: "Patterns", question: "Quais contratos devem ser tipados e testados primeiro?" },
  { id: "patterns-06", area: "Patterns", question: "Que partes devem ser configuráveis sem alterar código?" },
  { id: "patterns-07", area: "Patterns", question: "Onde o agente pode ajudar a operar ou validar o projeto?" },
  { id: "antipatterns-01", area: "Antipatterns", question: "Quais soluções você já sabe que não quer?" },
  { id: "antipatterns-02", area: "Antipatterns", question: "Quais atalhos costumam gerar retrabalho nesse tipo de projeto?" },
  { id: "antipatterns-03", area: "Antipatterns", question: "Que decisões podem criar lock-in ruim?" },
  { id: "antipatterns-04", area: "Antipatterns", question: "Onde uma UI confusa ou excesso de informação seria crítico?" },
  { id: "antipatterns-05", area: "Antipatterns", question: "Quais falhas silenciosas precisam ser impossíveis?" },
  { id: "antipatterns-06", area: "Antipatterns", question: "Que dependências ou abstrações seriam prematuras?" },
  { id: "antipatterns-07", area: "Antipatterns", question: "O que precisa ser explicitamente proibido na TechSpec?" },
  { id: "delivery-01", area: "Entrega", question: "Qual Definition of Done mínima para considerar pronto?" },
  { id: "delivery-02", area: "Entrega", question: "Quais testes automatizados precisam existir no MVP?" },
  { id: "delivery-03", area: "Entrega", question: "Quais validações manuais precisam de evidência?" },
  { id: "delivery-04", area: "Entrega", question: "Que documentação deve ser gerada junto do projeto?" },
  { id: "delivery-05", area: "Entrega", question: "Quais critérios bloqueiam materializar ou executar tasks?" },
  { id: "delivery-06", area: "Entrega", question: "Como priorizar as primeiras tasks?" },
  { id: "delivery-07", area: "Entrega", question: "Quais decisões precisam virar ADR?" },
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function text(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function parseQuestion(value: unknown): ProjectBlueprintQuestion | null {
  if (!isRecord(value)) return null;
  const id = text(value.id);
  const area = text(value.area);
  const question = text(value.question);
  if (!id || !area || !question) return null;
  return { id, area, question };
}

function parseTask(value: unknown): ProjectBlueprintTask | null {
  if (!isRecord(value)) return null;
  const title = text(value.title) || text(value.name) || text(value.task);
  if (!title) return null;
  const dependencies = Array.isArray(value.dependencies)
    ? value.dependencies
        .filter((item): item is string => typeof item === "string" && item.trim() !== "")
        .map((item) => item.trim())
    : undefined;
  return {
    title,
    body: text(value.body) || text(value.description) || text(value.details),
    dependencies,
  };
}

export function parseProjectBlueprintAgentResponse(
  content: string,
): ProjectBlueprintAgentResponse | null {
  const rawJson = extractJsonObject(content);
  if (!rawJson) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(rawJson);
  } catch {
    return null;
  }
  if (!isRecord(parsed)) return null;
  if (parsed.state === "question_batch") {
    const questions = Array.isArray(parsed.questions)
      ? parsed.questions.map(parseQuestion).filter((item): item is ProjectBlueprintQuestion => item !== null)
      : [];
    if (!questions.length || questions.length > PROJECT_BLUEPRINT_BATCH_SIZE) return null;
    return {
      state: "question_batch",
      questions,
      running_summary: text(parsed.running_summary),
      detected_subprojects: Array.isArray(parsed.detected_subprojects)
        ? parsed.detected_subprojects
            .filter((item): item is string => typeof item === "string" && item.trim() !== "")
            .map((item) => item.trim())
        : undefined,
    };
  }
  if (parsed.state === "final_plan") {
    const tasks = Array.isArray(parsed.tasks)
      ? parsed.tasks.map(parseTask).filter((item): item is ProjectBlueprintTask => item !== null)
      : [];
    const prd = text(parsed.prd);
    const techspec = text(parsed.techspec);
    const definitionOfDone = text(parsed.definition_of_done);
    if (!prd || !techspec || !definitionOfDone || !tasks.length) return null;
    return {
      state: "final_plan",
      running_summary: text(parsed.running_summary),
      detected_subprojects: Array.isArray(parsed.detected_subprojects)
        ? parsed.detected_subprojects
            .filter((item): item is string => typeof item === "string" && item.trim() !== "")
            .map((item) => item.trim())
        : [],
      prd,
      techspec,
      tasks,
      definition_of_done: definitionOfDone,
    };
  }
  return null;
}

export function projectBlueprintQuestionBatch(answeredCount: number) {
  return PROJECT_BLUEPRINT_QUESTION_BANK.slice(
    answeredCount,
    answeredCount + PROJECT_BLUEPRINT_BATCH_SIZE,
  );
}

export function buildProjectBlueprintPrompt(args: {
  title: string;
  idea: string;
  answers: ProjectBlueprintAnswer[];
  knowledgeSources: KnowledgeSource[];
}) {
  const selectedSources = args.knowledgeSources.length
    ? args.knowledgeSources.map((source) => `- ${source.name}: ${source.file_path}`).join("\n")
    : "- Nenhuma fonte selecionada.";
  const answers = args.answers.length
    ? args.answers
        .map(
          (answer, index) =>
            `${index + 1}. [${answer.area}] ${answer.question}\n   Resposta: ${answer.answer}`,
        )
        .join("\n")
    : "Nenhuma resposta ainda.";
  return [
    `Você é o agente planejador de um novo projeto do workspace chamado "${args.title}".`,
    "",
    "Objetivo: conduzir uma entrevista adaptativa e depois gerar PRD, TechSpec, Tasks e Definition of Done.",
    "Não escreva arquivos e não implemente código. A GUI vai persistir o resultado.",
    "",
    `Ideia inicial:\n${args.idea || "Sem ideia inicial detalhada."}`,
    "",
    `Base de conhecimento selecionada:\n${selectedSources}`,
    "",
    `Respostas já coletadas:\n${answers}`,
    "",
    "Banco de perguntas disponível (77 perguntas, escolha próximas perguntas conforme lacunas reais):",
    JSON.stringify(PROJECT_BLUEPRINT_QUESTION_BANK, null, 2),
    "",
    "Regras:",
    "- Pergunte em lotes de até 3 perguntas para usar cada resposta como insumo do próximo lote.",
    "- Use as respostas anteriores para refinar o próximo lote; não repita pergunta já respondida.",
    "- Cubra negócio, produto, stack, requisitos funcionais, requisitos não funcionais, patterns, antipatterns, multiprojeto, riscos e DoD.",
    "- Se identificar que o escopo pede mais de um projeto, preencha detected_subprojects.",
    "- Quando houver contexto suficiente, retorne o plano final completo.",
    "",
    "Responda SOMENTE JSON válido, sem markdown fora do JSON.",
    "Formato de pergunta:",
    JSON.stringify({
      state: "question_batch",
      running_summary: "...",
      detected_subprojects: ["opcional"],
      questions: [{ id: "business-01", area: "Negócio", question: "..." }],
    }),
    "Formato final:",
    JSON.stringify({
      state: "final_plan",
      running_summary: "...",
      detected_subprojects: [],
      prd: "# PRD ...",
      techspec: "# TechSpec ...",
      tasks: [{ title: "...", body: "...", dependencies: [] }],
      definition_of_done: "# Definition of Done ...",
    }),
  ].join("\n");
}

export function buildLocalProjectBlueprintPlan(args: {
  title: string;
  idea: string;
  answers: ProjectBlueprintAnswer[];
  knowledgeSources: KnowledgeSource[];
}): Extract<ProjectBlueprintAgentResponse, { state: "final_plan" }> {
  const grouped = args.answers.reduce<Record<string, ProjectBlueprintAnswer[]>>((acc, answer) => {
    acc[answer.area] = [...(acc[answer.area] ?? []), answer];
    return acc;
  }, {});
  const answerMarkdown = Object.entries(grouped)
    .map(
      ([area, answers]) =>
        `### ${area}\n${answers.map((answer) => `- ${answer.question}: ${answer.answer}`).join("\n")}`,
    )
    .join("\n\n");
  const knowledge = args.knowledgeSources.length
    ? args.knowledgeSources.map((source) => `- ${source.name}: ${source.file_path}`).join("\n")
    : "- Nenhuma fonte selecionada.";
  const summary = args.answers.length
    ? `Plano inicial derivado de ${args.answers.length} resposta(s) de entrevista.`
    : "Plano inicial gerado a partir da ideia informada.";
  return {
    state: "final_plan",
    running_summary: summary,
    detected_subprojects: detectSubprojects(args.answers),
    prd: [
      `# PRD - ${args.title}`,
      "",
      "## Ideia",
      args.idea || "Sem ideia detalhada.",
      "",
      "## Respostas da entrevista",
      answerMarkdown || "Ainda sem respostas registradas.",
      "",
      "## Base de conhecimento",
      knowledge,
    ].join("\n"),
    techspec: [
      `# TechSpec - ${args.title}`,
      "",
      "## Stack e arquitetura",
      "A stack final deve ser confirmada pelo agente a partir da entrevista e da base de conhecimento.",
      "",
      "## Contratos",
      "Definir APIs, scripts, entidades e integrações antes da execução.",
      "",
      "## Riscos",
      "Registrar decisões arquiteturais abertas como ADR quando necessário.",
    ].join("\n"),
    tasks: [
      {
        title: "Fechar PRD e TechSpec do novo projeto",
        body: "Revisar as respostas, confirmar escopo único ou multiprojeto e aprovar os artefatos.",
      },
      {
        title: "Criar scaffold e contratos iniciais",
        body: "Criar estrutura do projeto, scripts base, contratos de dados/API e configuração inicial.",
        dependencies: ["Fechar PRD e TechSpec do novo projeto"],
      },
      {
        title: "Implementar fluxo principal do MVP",
        body: "Construir a primeira versão funcional com testes do fluxo crítico.",
        dependencies: ["Criar scaffold e contratos iniciais"],
      },
      {
        title: "Validar DoD e evidências",
        body: "Rodar testes, revisão, QA manual quando necessário e registrar evidências.",
        dependencies: ["Implementar fluxo principal do MVP"],
      },
    ],
    definition_of_done: [
      "# Definition of Done",
      "",
      "- PRD, TechSpec e tasks aprovados.",
      "- Projeto registrado no workspace.",
      "- Scripts de dev/test/build definidos.",
      "- Testes relevantes passando.",
      "- Evidências registradas no Workbench.",
    ].join("\n"),
  };
}

function detectSubprojects(answers: ProjectBlueprintAnswer[]) {
  const text = answers.map((answer) => answer.answer).join(" ").toLowerCase();
  if (/\b(api|backend)\b/.test(text) && /\b(frontend|web|ui)\b/.test(text)) {
    return ["frontend", "backend"];
  }
  if (/\b(cli)\b/.test(text) && /\b(api|backend|web|frontend)\b/.test(text)) {
    return ["app", "cli"];
  }
  return [];
}
