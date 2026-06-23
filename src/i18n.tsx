/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { api } from "./tauri";

export type Locale = "en" | "pt-BR";

export const DEFAULT_LOCALE: Locale = "en";
export const LOCALE_APP_STATE_KEY = "ui.locale";
export const LOCALE_STORAGE_KEY = "clia.locale";

const EN = {
  "app.name": "clia.dev",
  "app.tagline": "ADE",
  "app.initializing": "Initializing clia.dev",
  "app.bootstrapErrorLog": "clia.dev failed during React bootstrap",
  "about.open": "Open clia.dev status",
  "about.openHint": "Double-click to open clia.dev status",
  "about.eyebrow": "Status",
  "about.title": "clia.dev",
  "about.description":
    "Agent development environment for workspaces, code, machines, deploys, and reusable skills.",
  "about.version": "Version",
  "about.workspace": "Workspace",
  "about.project": "Project",
  "about.flow": "Flow",
  "about.language": "Language",
  "about.agent": "Agent",
  "about.agentWorking": "working",
  "about.agentIdle": "idle",
  "about.runtime": "Runtime",
  "fatal.eyebrow": "React bootstrap error",
  "fatal.title": "The interface failed before opening.",
  "fatal.reload": "Reload",
  "nav.queue": "My queue",
  "nav.knowledge": "Knowledge base",
  "nav.code": "Code",
  "nav.git": "Git",
  "nav.deploy": "Deploy",
  "nav.skills": "Capabilities",
  "nav.agents": "Agents",
  "nav.settings": "Workspace settings",
  "common.refresh": "Refresh",
  "common.close": "Close",
  "common.cancel": "Cancel",
  "common.save": "Save",
  "common.remove": "Remove",
  "common.create": "Create",
  "common.edit": "Edit",
  "common.import": "Import",
  "common.export": "Export",
  "common.loading": "loading",
  "common.none": "None",
  "common.noColor": "No color",
  "topbar.newProject": "New project",
  "topbar.newCard": "New card",
  "topbar.newWorkspace": "New workspace",
  "topbar.activeContext": "Active context",
  "topbar.switchWorkspace": "Switch workspace",
  "topbar.switchProject": "Switch project",
  "topbar.noWorkspace": "No workspace",
  "topbar.noProject": "No project",
  "topbar.archived": "Archived ({count})",
  "topbar.agentWorking": "agent working",
  "workspace.modal.title": "Workspace",
  "workspace.modal.description": "Choose a folder for clia.dev workspace data and artifacts.",
  "workspace.modal.name": "Name",
  "workspace.modal.root": "Root folder",
  "workspace.modal.pick": "Choose",
  "workspace.modal.save": "Save workspace",
  "workspace.import.title": "Import workspace",
  "workspace.import.description":
    "Choose the .wksdw package and the folder where repositories will be cloned.",
  "workspace.import.file": ".wksdw file",
  "workspace.import.name": "Name",
  "workspace.import.placeholderName": "Package name",
  "workspace.import.destination": "Destination folder",
  "workspace.import.submit": "Import and clone projects",
  "workspace.import.empty": "No project declared in the package.",
  "workspace.settings.eyebrow": "Workspace",
  "workspace.settings.title": "Workspace settings",
  "workspace.settings.color.title": "Workspace color",
  "workspace.settings.color.description": "Choose one of 8 schemes to identify this workspace.",
  "workspace.settings.color.remove": "Remove color",
  "workspace.settings.theme.title": "Theme",
  "workspace.settings.theme.description":
    "Choose the global app theme. Workspace color still controls the active workspace accents.",
  "workspace.settings.theme.clia": "CLIA",
  "workspace.settings.theme.black": "Black",
  "workspace.settings.editor.title": "Code editor",
  "workspace.settings.editor.description": "Editor font size (applies immediately).",
  "workspace.settings.editorPx": "Editor font size in pixels",
  "workspace.settings.rtk.title": "Token savings with RTK",
  "workspace.settings.rtk.description":
    "Enable RTK per agent profile and run guided setup for provider hooks. Telemetry is always blocked by clia.dev.",
  "workspace.settings.rtk.enable": "Enable",
  "workspace.settings.rtk.disable": "Disable",
  "workspace.settings.rtk.install": "Check and install",
  "workspace.settings.rtk.checkInstall": "Check installed",
  "workspace.settings.rtk.setup": "Setup hooks",
  "workspace.settings.rtk.refresh": "Refresh RTK status",
  "workspace.settings.rtk.enabled": "RTK enabled",
  "workspace.settings.rtk.disabled": "RTK disabled",
  "workspace.settings.rtk.checking": "Checking RTK...",
  "workspace.settings.rtk.emptyAgents": "No agent profile configured yet.",
  "workspace.settings.language.title": "Language",
  "workspace.settings.language.description":
    "Default is English. Development, product, and tech keywords stay in English.",
  "workspace.settings.language.en": "English",
  "workspace.settings.language.ptBR": "Português (Brasil)",
  "workspace.settings.cards.title": "Card IDs",
  "workspace.settings.cards.description": "Configure each project's prefix and next number.",
  "workspace.settings.prefix": "Prefix",
  "workspace.settings.nextNumber": "Next number",
  "workspace.settings.save": "Save settings",
  "workspace.settings.emptyProjects": "No project registered in this workspace.",
  "welcome.title": "Agent development workspace",
  "welcome.body":
    "Create or import a workspace to manage projects, cards, flows, agents, machines, deploy packages, and knowledge sources.",
  "welcome.createWorkspace": "Create workspace",
  "welcome.importWorkspace": "Import workspace",
  "welcome.savedWorkspaces": "{count} saved workspace(s)",
  "welcome.noWorkspace": "No saved workspace",
  "welcome.noteFlow": "Cards move through PRD, execution, QA, and PR.",
  "welcome.noteKnowledge": "Attachments and context stay with the requirement.",
  "project.modal.title": "Add project",
  "project.modal.local": "Local project",
  "project.modal.clone": "Clone repository",
  "project.modal.name": "Name",
  "project.modal.path": "Path",
  "project.modal.remote": "Remote URL",
  "project.modal.addLocal": "Add local project",
  "project.modal.cloneSubmit": "Clone project",
  "knowledge.title": "Knowledge base",
  "knowledge.scope": "Workspace",
  "knowledge.summary":
    "{sources} source(s) · {blueprints} blueprint(s) · {questions} base questions · batch up to {batch}",
  "knowledge.sources": "Sources",
  "knowledge.projects": "Projects",
  "knowledge.attachments": "Attachments",
  "knowledge.blueprints": "Blueprints",
  "knowledge.attachWorkspace": "Attach to workspace",
  "knowledge.attachProject": "Attach to project",
  "knowledge.noAttachments": "No attachment",
  "knowledge.addFiles": "Add workspace or active project files.",
  "knowledge.noBlueprint": "No blueprint",
  "knowledge.removeSource": "Remove {name}",
  "knowledge.removeSourceTitle": "Remove source",
  "knowledge.removeSourceBody":
    'Remove "{name}" from the Knowledge base? The managed file under .dw/knowledge will also be removed.',
  "blueprint.new": "New project",
  "blueprint.initialInterview": "Initial interview",
  "blueprint.agent": "Agent: {name}",
  "blueprint.noAgent": "No active agent",
  "blueprint.answerCount": "{count} answer(s)",
  "blueprint.name": "Name",
  "blueprint.namePlaceholder": "New project name",
  "blueprint.idea": "Idea",
  "blueprint.ideaPlaceholder":
    "Describe the project, problem, expected stack, and known constraints.",
  "blueprint.startInterview": "Start interview",
  "blueprint.answers": "answers",
  "blueprint.sources": "sources",
  "blueprint.projects": "projects",
  "blueprint.tasks": "tasks",
  "blueprint.waiting": "Waiting for agent...",
  "blueprint.generateNow": "Generate plan now",
  "blueprint.sendBatch": "Send batch",
  "blueprint.materialize": "Materialize project",
  "blueprint.materialized": "materialized",
  "blueprint.noSource": "No source added.",
  "blueprint.status.draft": "Draft",
  "blueprint.status.interviewing": "Interview",
  "blueprint.status.planned": "Planned",
  "blueprint.status.materialized": "Materialized",
  "blueprint.status.archived": "Archived",
  "flows.title": "Cloud workflows",
  "flows.scope": "Flows",
  "flows.description":
    "Workflows are authored in the portal and cached locally so the desktop can run them.",
  "flows.noProject": "Select a project to configure flows.",
  "flows.noWorkspace": "Select a workspace to load cloud workflows.",
  "flows.installed": "Cached locally",
  "flows.active": "Active",
  "flows.default": "Default",
  "flows.makeActive": "Make active",
  "flows.manageCloud": "Manage in portal",
  "flows.cloudOnly": "Cloud-only authoring",
  "flows.cloudOnlyDescription":
    "Create, edit, publish, archive, and assign workflows in the portal. The desktop only keeps a read-only cache.",
  "flows.addPreset": "Add from preset",
  "flows.createMaintain": "Create and maintain",
  "flows.custom": "Create custom flow",
  "flows.fromUrl": "Create flow from URL (agent)",
  "flows.reset": "Reset to dev-workflow only",
  "skills.title": "Workspace capabilities",
  "skills.scope": "Capabilities",
  "skills.description":
    "Manage frameworks, flows, and skills once per workspace. Use /skill-name request to inject a skill into the agent.",
  "skills.import": "Import content",
  "skills.export": "Export .wksdw",
  "skills.frameworks": "Frameworks / flows",
  "skills.install": "Install skill",
  "skills.search": "Search",
  "skills.noWorkspace": "Select a workspace to manage skills.",
  "skills.installed": "installed",
  "skills.available": "available",
  "skills.workspaceBase": "workspace base",
  "skills.installAction": "Install",
  "skills.workspaceNotSelected": "Workspace not selected",
  "skills.installedDiscovered": "Installed and discovered skills",
  "skills.noDescription": "No description in SKILL.md",
  "skills.exportable": "exportable",
  "skills.empty": "No skill installed in the workspace.",
} as const;

const PT_BR: Record<TranslationKey, string> = {
  "app.name": "clia.dev",
  "app.tagline": "ADE",
  "app.initializing": "Inicializando clia.dev",
  "app.bootstrapErrorLog": "clia.dev falhou no bootstrap do React",
  "about.open": "Abrir status do clia.dev",
  "about.openHint": "Duplo clique para abrir o status do clia.dev",
  "about.eyebrow": "Status",
  "about.title": "clia.dev",
  "about.description":
    "Agent development environment para workspaces, code, machines, deploys e reusable skills.",
  "about.version": "Versão",
  "about.workspace": "Workspace",
  "about.project": "Project",
  "about.flow": "Flow",
  "about.language": "Language",
  "about.agent": "Agent",
  "about.agentWorking": "working",
  "about.agentIdle": "idle",
  "about.runtime": "Runtime",
  "fatal.eyebrow": "Erro no bootstrap do React",
  "fatal.title": "A interface falhou antes de abrir.",
  "fatal.reload": "Recarregar",
  "nav.queue": "Minha fila",
  "nav.knowledge": "Knowledge base",
  "nav.code": "Code",
  "nav.git": "Git",
  "nav.deploy": "Deploy",
  "nav.skills": "Capabilities",
  "nav.agents": "Agents",
  "nav.settings": "Workspace settings",
  "common.refresh": "Refresh",
  "common.close": "Fechar",
  "common.cancel": "Cancelar",
  "common.save": "Salvar",
  "common.remove": "Remover",
  "common.create": "Criar",
  "common.edit": "Editar",
  "common.import": "Importar",
  "common.export": "Exportar",
  "common.loading": "carregando",
  "common.none": "Nenhum",
  "common.noColor": "Sem cor",
  "topbar.newProject": "Novo project",
  "topbar.newCard": "Novo card",
  "topbar.newWorkspace": "Novo workspace",
  "topbar.activeContext": "Contexto ativo",
  "topbar.switchWorkspace": "Trocar workspace",
  "topbar.switchProject": "Trocar project",
  "topbar.noWorkspace": "Sem workspace",
  "topbar.noProject": "Sem project",
  "topbar.archived": "Archived ({count})",
  "topbar.agentWorking": "agent working",
  "workspace.modal.title": "Workspace",
  "workspace.modal.description":
    "Escolha uma pasta para os dados e artifacts do workspace clia.dev.",
  "workspace.modal.name": "Nome",
  "workspace.modal.root": "Root folder",
  "workspace.modal.pick": "Escolher",
  "workspace.modal.save": "Salvar workspace",
  "workspace.import.title": "Importar workspace",
  "workspace.import.description":
    "Escolha o pacote .wksdw e a pasta onde os repositories serão clonados.",
  "workspace.import.file": "Arquivo .wksdw",
  "workspace.import.name": "Nome",
  "workspace.import.placeholderName": "Nome do pacote",
  "workspace.import.destination": "Pasta destino",
  "workspace.import.submit": "Importar e clonar projects",
  "workspace.import.empty": "Nenhum project declarado no pacote.",
  "workspace.settings.eyebrow": "Workspace",
  "workspace.settings.title": "Workspace settings",
  "workspace.settings.color.title": "Cor do workspace",
  "workspace.settings.color.description":
    "Escolha um dos 8 schemes para identificar este workspace.",
  "workspace.settings.color.remove": "Remover cor",
  "workspace.settings.theme.title": "Theme",
  "workspace.settings.theme.description":
    "Escolha o theme global do app. A cor do workspace ainda controla os accents do workspace ativo.",
  "workspace.settings.theme.clia": "CLIA",
  "workspace.settings.theme.black": "Black",
  "workspace.settings.editor.title": "Code editor",
  "workspace.settings.editor.description": "Tamanho da fonte do editor (aplica imediatamente).",
  "workspace.settings.editorPx": "Tamanho da fonte do editor em pixels",
  "workspace.settings.rtk.title": "Token savings with RTK",
  "workspace.settings.rtk.description":
    "Habilite RTK por agent profile e rode o setup guiado dos hooks do provider. A telemetry fica sempre bloqueada pelo clia.dev.",
  "workspace.settings.rtk.enable": "Habilitar",
  "workspace.settings.rtk.disable": "Desabilitar",
  "workspace.settings.rtk.install": "Checar e instalar",
  "workspace.settings.rtk.checkInstall": "Checar instalado",
  "workspace.settings.rtk.setup": "Setup hooks",
  "workspace.settings.rtk.refresh": "Atualizar status do RTK",
  "workspace.settings.rtk.enabled": "RTK habilitado",
  "workspace.settings.rtk.disabled": "RTK desabilitado",
  "workspace.settings.rtk.checking": "Checando RTK...",
  "workspace.settings.rtk.emptyAgents": "Nenhum agent profile configurado ainda.",
  "workspace.settings.language.title": "Language",
  "workspace.settings.language.description":
    "O default é English. Keywords de desenvolvimento, produto e tech ficam em English.",
  "workspace.settings.language.en": "English",
  "workspace.settings.language.ptBR": "Português (Brasil)",
  "workspace.settings.cards.title": "IDs dos cards",
  "workspace.settings.cards.description": "Configure o prefixo e o próximo número de cada project.",
  "workspace.settings.prefix": "Prefixo",
  "workspace.settings.nextNumber": "Próximo número",
  "workspace.settings.save": "Salvar settings",
  "workspace.settings.emptyProjects": "Nenhum project registrado neste workspace.",
  "welcome.title": "Agent development workspace",
  "welcome.body":
    "Crie ou importe um workspace para gerir projects, cards, flows, agents, machines, deploy packages e knowledge sources.",
  "welcome.createWorkspace": "Criar workspace",
  "welcome.importWorkspace": "Importar workspace",
  "welcome.savedWorkspaces": "{count} workspace(s) salvos",
  "welcome.noWorkspace": "Nenhum workspace salvo",
  "welcome.noteFlow": "Cards passam por PRD, execução, QA e PR.",
  "welcome.noteKnowledge": "Anexos e contexto ficam junto do requisito.",
  "project.modal.title": "Adicionar project",
  "project.modal.local": "Project local",
  "project.modal.clone": "Clonar repository",
  "project.modal.name": "Nome",
  "project.modal.path": "Path",
  "project.modal.remote": "Remote URL",
  "project.modal.addLocal": "Adicionar project local",
  "project.modal.cloneSubmit": "Clonar project",
  "knowledge.title": "Knowledge base",
  "knowledge.scope": "Workspace",
  "knowledge.summary":
    "{sources} source(s) · {blueprints} blueprint(s) · {questions} perguntas base · lote até {batch}",
  "knowledge.sources": "Sources",
  "knowledge.projects": "Projects",
  "knowledge.attachments": "Anexos",
  "knowledge.blueprints": "Blueprints",
  "knowledge.attachWorkspace": "Anexar ao workspace",
  "knowledge.attachProject": "Anexar ao project",
  "knowledge.noAttachments": "Nenhum anexo",
  "knowledge.addFiles": "Adicione arquivos do workspace ou do project ativo.",
  "knowledge.noBlueprint": "Nenhum blueprint",
  "knowledge.removeSource": "Remover {name}",
  "knowledge.removeSourceTitle": "Remover source",
  "knowledge.removeSourceBody":
    'Remover "{name}" da Knowledge base? O arquivo gerenciado em .dw/knowledge também será removido.',
  "blueprint.new": "Novo project",
  "blueprint.initialInterview": "Entrevista inicial",
  "blueprint.agent": "Agent: {name}",
  "blueprint.noAgent": "Sem agent ativo",
  "blueprint.answerCount": "{count} resposta(s)",
  "blueprint.name": "Nome",
  "blueprint.namePlaceholder": "Nome do novo project",
  "blueprint.idea": "Ideia",
  "blueprint.ideaPlaceholder":
    "Descreva o project, problema, stack esperada e restrições conhecidas.",
  "blueprint.startInterview": "Iniciar entrevista",
  "blueprint.answers": "respostas",
  "blueprint.sources": "sources",
  "blueprint.projects": "projects",
  "blueprint.tasks": "tasks",
  "blueprint.waiting": "Aguardando agent...",
  "blueprint.generateNow": "Gerar plano agora",
  "blueprint.sendBatch": "Enviar lote",
  "blueprint.materialize": "Materializar project",
  "blueprint.materialized": "materializado",
  "blueprint.noSource": "Nenhuma source adicionada.",
  "blueprint.status.draft": "Draft",
  "blueprint.status.interviewing": "Entrevista",
  "blueprint.status.planned": "Planejado",
  "blueprint.status.materialized": "Materializado",
  "blueprint.status.archived": "Arquivado",
  "flows.title": "Fluxos da cloud",
  "flows.scope": "Flows",
  "flows.description":
    "Os fluxos são criados no portal e ficam em cache local para o desktop executar.",
  "flows.noProject": "Selecione um project para configurar flows.",
  "flows.noWorkspace": "Selecione um workspace para carregar fluxos da cloud.",
  "flows.installed": "Cache local",
  "flows.active": "Ativo",
  "flows.default": "Default",
  "flows.makeActive": "Tornar ativo",
  "flows.manageCloud": "Gerenciar no portal",
  "flows.cloudOnly": "Autoria somente na cloud",
  "flows.cloudOnlyDescription":
    "Crie, edite, publique, arquive e atribua fluxos no portal. O desktop mantém apenas um cache read-only.",
  "flows.addPreset": "Adicionar de um preset",
  "flows.createMaintain": "Criar e manter",
  "flows.custom": "Criar flow custom",
  "flows.fromUrl": "Criar flow de URL (com agent)",
  "flows.reset": "Resetar para só o dev-workflow",
  "skills.title": "Workspace capabilities",
  "skills.scope": "Capabilities",
  "skills.description":
    "Gerencie frameworks, flows e skills uma vez por workspace. Use /nome-da-skill pedido para injetar a skill diretamente no agent.",
  "skills.import": "Importar conteúdo",
  "skills.export": "Exportar .wksdw",
  "skills.frameworks": "Frameworks / flows",
  "skills.install": "Instalar skill",
  "skills.search": "Buscar",
  "skills.noWorkspace": "Selecione um workspace para gerenciar skills.",
  "skills.installed": "instalado",
  "skills.available": "disponível",
  "skills.workspaceBase": "base do workspace",
  "skills.installAction": "Instalar",
  "skills.workspaceNotSelected": "Workspace não selecionado",
  "skills.installedDiscovered": "Skills instaladas e descobertas",
  "skills.noDescription": "Sem descrição no SKILL.md",
  "skills.exportable": "exportável",
  "skills.empty": "Nenhuma skill instalada no workspace.",
};

export type TranslationKey = keyof typeof EN;

type I18nContextValue = {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function isLocale(value: string | null | undefined): value is Locale {
  return value === "en" || value === "pt-BR";
}

export function normalizeLocale(value: string | null | undefined): Locale {
  return isLocale(value) ? value : DEFAULT_LOCALE;
}

export function translate(
  locale: Locale,
  key: TranslationKey,
  params: Record<string, string | number> = {},
): string {
  const dictionary = locale === "pt-BR" ? PT_BR : EN;
  let value = dictionary[key] ?? EN[key] ?? key;
  for (const [param, replacement] of Object.entries(params)) {
    value = value.replaceAll(`{${param}}`, String(replacement));
  }
  return value;
}

function readInitialLocale(): Locale {
  if (typeof window === "undefined") return DEFAULT_LOCALE;
  return normalizeLocale(window.localStorage.getItem(LOCALE_STORAGE_KEY));
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(readInitialLocale);

  useEffect(() => {
    let disposed = false;
    void api.getAppState(LOCALE_APP_STATE_KEY).then((result) => {
      if (disposed || !result.ok || !isLocale(result.value)) return;
      setLocaleState(result.value);
      window.localStorage.setItem(LOCALE_STORAGE_KEY, result.value);
    });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  const setLocale = useCallback((nextLocale: Locale) => {
    setLocaleState(nextLocale);
    window.localStorage.setItem(LOCALE_STORAGE_KEY, nextLocale);
    void api.setAppState(LOCALE_APP_STATE_KEY, nextLocale);
  }, []);

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      setLocale,
      t: (key, params) => translate(locale, key, params),
    }),
    [locale, setLocale],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used inside I18nProvider");
  }
  return context;
}
