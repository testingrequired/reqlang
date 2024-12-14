import {
  HttpRequest,
  HttpResponse,
  UnresolvedRequestFile,
} from "reqlang-types";
import * as RsResult from "rsresult";

/**
 * State for an individual request file
 */
export type ReqlangWorkspaceFileState = {
  /**
   * Current selected environment
   */
  env: string | null;
  parsedReqfile: RsResult.Result<SimplifiedParsedRequestFile> | null;
  isWaitingForResponse: boolean;
  lastResponse: HttpResponse | null;
};

export type ParseNotification = {
  file_id: string;
  result: RsResult.Result<SimplifiedParsedRequestFile>;
};

/**
 * Simplified version of a parsed request file for use in the VSC extension.
 *
 * This is sent from the language server to VSC.
 */
export type SimplifiedParsedRequestFile = {
  vars: string[];
  envs: string[];
  prompts: string[];
  secrets: string[];
  request: HttpRequest;
  full: UnresolvedRequestFile;
};

export type ExecuteRequestParams = {
  uri: string;
  env: string;
  vars: Record<string, string>;
  prompts: Record<string, string>;
  secrets: Record<string, string>;
};

export type ExportRequestParams = {
  uri: string;
  env: string;
  vars: Record<string, string>;
  prompts: Record<string, string>;
  secrets: Record<string, string>;
  format: string;
};

/**
 * The possible choices for the Reqlang Menu in the VSC extension
 */
export enum MenuChoices {
  PickEnv = "Pick an environment",
  ClearEnv = "Clear the environment",
  RunRequest = "Run request",
  ExportRequest = "Export request as curl script",
  StartLanguageServer = "Start Language Server",
  StopLanguageServer = "Stop Language Server",
  RestartLanguageServer = "Restart Language Server",
  OpenOutput = "Open Output Channel",
}

/**
 * The possible choices for the Reqlang Menu in the VSC extension
 */
export enum Commands {
  PickEnv = "reqlang.pickEnv",
  ClearEnv = "reqlang.clearEnv",
  RunRequest = "reqlang.run",
  Menu = "reqlang.menu",
  Execute = "reqlang.executeRequest",
  Export = "reqlang.exportRequest",
  StartLanguageServer = "reqlang.startLanguageServer",
  StopLanguageServer = "reqlang.stopLanguageServer",
  RestartLanguageServer = "reqlang.restartLanguageServer",
  Install = "reqlang.install",
  OpenMdnDocsHttp = "reqlang.openMdnDocsHttp",
  OpenMdnDocsHttpMessages = "reqlang.openMdnDocsHttpMessages",
  OpenMdnDocsHttpSpecs = "reqlang.openMdnDocsHttpSpecs",
  ExportToFile = "reqlang.exportToFile",
  DebugResetWorkspaceState = "reqlang.debugResetWorkspaceState",
}
