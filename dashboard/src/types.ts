export type Language = "typescript" | "javascript" | "rust" | "python";

export type SymbolKind =
  | "function"
  | "class"
  | "interface"
  | "struct"
  | "enum"
  | "trait"
  | "component"
  | "variable"
  | "constant"
  | "method";

export type ArchitectureKind =
  | "api"
  | "services"
  | "repositories"
  | "database"
  | "workers"
  | "components"
  | "hooks"
  | "pages"
  | "external"
  | "other";

export type ArchitectureGroup = "frontend" | "backend" | "infrastructure" | "other";

export interface RepositorySummary {
  id: string;
  name: string;
  url: string;
  defaultBranch: string;
  commitSha: string;
  lineCount: number;
}

export interface IndexStats {
  fileCount: number;
  symbolCount: number;
  dependencyCount: number;
  lineCount: number;
}

export interface ArchitectureLayer {
  kind: ArchitectureKind;
  fileCount: number;
}

export interface ArchitectureGroupNode {
  name: string;
  group: ArchitectureGroup;
  fileCount: number;
  layers: ArchitectureLayer[];
}

export interface ArchitectureOutline {
  groups: ArchitectureGroupNode[];
  summary: string;
}

export interface RepoHealth {
  fileCount: number;
  lineCount: number;
  languageCount: number;
  symbolCount: number;
  dependencyCount: number;
  apiEndpointCount: number;
  testFileCount: number;
  testSymbolCount: number;
  architectureMapped: number;
  complexity: number;
  testPresence: number;
}

export interface LanguageCount {
  language: Language;
  fileCount: number;
  percent: number;
}

export interface OverviewResponse {
  repository: RepositorySummary;
  stats: IndexStats;
  architecture: ArchitectureLayer[];
  architectureTree: ArchitectureOutline;
  health: RepoHealth;
  languages: LanguageCount[];
  aiSummary: string;
}

export interface FileContents {
  path: string;
  language: Language;
  content: string;
}

export interface FileResponse {
  id: string;
  path: string;
  language: Language;
  sizeBytes: number;
}

export interface SearchHit {
  id: string;
  name: string;
  path: string;
  kind: string;
  score: number;
  source: "symbolName" | "fileName" | "path" | "text" | "semantic";
}

export interface SymbolResponse {
  id: string;
  fileId: string;
  name: string;
  kind: SymbolKind;
  startLine: number;
  endLine: number;
  path: string;
}

export interface GraphNode {
  id: string;
  name: string;
  kind: string;
  fileId: string;
  path: string;
  x: number;
  y: number;
  layer: number;
}

export interface GraphEdge {
  source: string;
  target: string;
  kind: string;
}

export interface GraphResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
  focus?: string;
}

export type ImpactRole = "test" | "route" | "component" | "other";

export interface ImpactSymbol {
  id: string;
  name: string;
  kind: string;
  fileId: string;
  path: string;
  depth: number;
}

export interface ImpactedFile {
  path: string;
  fileId: string;
  symbolCount: number;
  roles: ImpactRole[];
}

export interface ImpactTreeNode {
  path: string;
  fileId: string;
  roles: ImpactRole[];
  children: ImpactTreeNode[];
}

export interface ImpactNarrative {
  text: string;
  citations: Citation[];
  mode: "retrieved" | "model";
}

export interface ImpactResponse {
  origin: ImpactSymbol;
  fileCount: number;
  symbolCount: number;
  direct: ImpactSymbol[];
  indirect: ImpactSymbol[];
  tests: ImpactSymbol[];
  testFileCount: number;
  routes: ImpactSymbol[];
  components: ImpactSymbol[];
  files: ImpactedFile[];
  tree: ImpactTreeNode;
  subgraph: GraphResponse;
  narrative: ImpactNarrative;
}

export interface SymbolRef {
  id: string;
  name: string;
  kind: string;
  path: string;
  relation: string;
}

export interface ExplainResponse {
  symbol: SymbolResponse;
  purpose: string;
  flow: string[];
  dependencies: SymbolRef[];
  dependents: SymbolRef[];
  citations: Citation[];
  mode: "retrieved" | "model";
  text: string;
}

export interface Citation {
  fileId: string;
  symbolId: string;
  symbol: string;
  path: string;
  startLine: number;
  endLine: number;
  excerpt: string;
}

export interface AskResponse {
  question: string;
  intent: "locate" | "explain" | "impact" | "evolution" | "search";
  mode: "retrieved" | "model";
  answer: string;
  citations: Citation[];
  confidence: "high" | "medium" | "low";
  retrievalSummary: string;
}

export interface CommitSummary {
  sha: string;
  authorName: string;
  authorEmail: string;
  authoredAt: number;
  subject: string;
  filesChanged: number;
}

export interface BranchSummary {
  name: string;
  sha: string;
  isDefault: boolean;
}

export interface AuthorSummary {
  name: string;
  email: string;
  commitCount: number;
  fileCount: number;
}

export interface FileHotspot {
  path: string;
  changeCount: number;
  contributorCount: number;
  lastCommitSha: string;
  lastAuthoredAt: number;
}

export interface GitEvolution {
  commits: CommitSummary[];
  branches: BranchSummary[];
  authors: AuthorSummary[];
  hotspots: FileHotspot[];
}

export interface IndexProgress {
  stage:
    | "queued"
    | "scanning"
    | "parsing"
    | "resolving"
    | "embedding"
    | "persisting"
    | "succeeded"
    | "failed";
  message: string;
  filesDiscovered: number;
  filesParsed: number;
  filesSkipped: number;
  symbolsExtracted: number;
  relationshipsBuilt: number;
  chunksEmbedded: number;
  durationMs: number;
}

export interface IndexJob {
  jobId: string;
  repositoryId?: string;
  status: IndexProgress["stage"];
  progress: IndexProgress;
  error?: string;
}

export type WorkstationView =
  | "overview"
  | "files"
  | "symbols"
  | "graph"
  | "search"
  | "ai"
  | "insights"
  | "evolution";
