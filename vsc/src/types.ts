import {
  HttpRequest,
  HttpResponse,
  UnresolvedRequestFile,
  RequestParamsFromClient,
} from "reqlang-types";
import * as RsResult from "rsresult";

/**
 * Responses from executed requests send to the language server
 */
export type RequestToBeExecuted = {
  startDateIso: string;
  params: RequestParamsFromClient;
  response: HttpResponse;
  endDateIso: string;
  wasSuccessful: boolean;
};

/**
 * State for an individual request file
 */
export type ReqfileState = {
  /**
   * Current selected environment
   */
  env: string | null;
  /**
   * Parsed request file from the server.
   */
  parsedReqfileFromServer: RsResult.Result<ParsedReqfileFromServer> | null;
  /**
   * If the latest request execution is waiting for a response
   */
  isWaitingForResponse: boolean;
  /**
   * List of request executions sent to the server
   */
  requestExecutions: RequestToBeExecuted[];
};

/**
 * A notification sent from the language server that a request file has been parsed
 */
export type ParseNotificationFromServer = {
  file_id: string;
  result: RsResult.Result<ParsedReqfileFromServer>;
};

/**
 * Parsed request file from the server.
 *
 * This is a simplified version of the actual parsed request file.
 */
export type ParsedReqfileFromServer = {
  /**
   * List of environment names in the request file
   */
  envs: string[];

  /**
   * List of variables names declared in the request file
   */
  vars: string[];

  /**
   * List of prompt names declared in the request file
   */
  prompts: string[];

  /**
   * List of secret names declared in the request file
   */
  secrets: string[];

  /**
   * HTTP Request object that still contains template references
   */
  request: HttpRequest;

  /**
   * Full parsed request file
   */
  full: UnresolvedRequestFile;
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
  LastResponse = "Last response",
  DebugClearWorkspaceState = "Debug: Clear Workspace State For This Request File",
}
