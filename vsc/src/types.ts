import { HttpRequest } from "reqlang-types";
import * as RsResult from "rsresult";

/**
 * State for an individual request file
 */
export type ReqlangWorkspaceFileState = {
  /**
   * Current selected environment
   */
  env: string | null;
  parseResult: RsResult.Result<ParseResult> | null;
};

export type ParseNotification = {
  file_id: string;
  result: RsResult.Result<ParseResult>;
};

export type ParseResult = {
  vars: string[];
  envs: string[];
  prompts: string[];
  secrets: string[];
  request: HttpRequest;
};

export type ExecuteRequestParams = {
  uri: string;
  env: string;
  vars: Record<string, string>;
  prompts: Record<string, string>;
  secrets: Record<string, string>;
};

/**
 * The possible choices for the Reqlang Menu in the VSC extension
 */
export enum MenuChoices {
  PickEnv = "Pick an environment",
  ClearEnv = "Clear the environment",
  RunRequest = "Run request",
  StartLanguageServer = "Start Language Server",
  StopLanguageServer = "Stop Language Server",
  RestartLanguageServer = "Restart Language Server",
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
  StartLanguageServer = "reqlang.startLanguageServer",
  StopLanguageServer = "reqlang.stopLanguageServer",
  RestartLanguageServer = "reqlang.restartLanguageServer",
  Install = "reqlang.install",
  OpenMdnDocsHttp = "reqlang.openMdnDocsHttp",
  OpenMdnDocsHttpMessages = "reqlang.openMdnDocsHttpMessages",
  OpenMdnDocsHttpSpecs = "reqlang.openMdnDocsHttpSpecs",
  ExportToFile = "reqlang.exportToFile",
}
