"use strict";
import { ExtensionContext, window } from "vscode";
import {
  type RequestToBeExecuted,
  type ReqfileState,
  type ParsedReqfileFromServer,
  RequestToBeExecutedParams,
} from "./types";
import * as RsResult from "rsresult";
import { updateStatusText } from "./status";
import { HttpResponse } from "reqlang-types";

/**
 * Set a request file's workspace state with it's parsed request file
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @param result The result of parsing the request file from language server
 * @returns Updated workspace state with parsed request file
 */
export function setParseResult(
  fileKey: string,
  context: ExtensionContext,
  result: RsResult.Result<ParsedReqfileFromServer>
): ReqfileState {
  return updateState(fileKey, context, (state) => {
    state.parsedReqfileFromServer = result;
    return state;
  });
}

/**
 * Get the parsed request file of the request file
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @returns The parsed request file of the request file
 */
export function getParseResults(
  fileKey: string,
  context: ExtensionContext
): RsResult.Result<ParsedReqfileFromServer> | null {
  const state = getOrInitState(fileKey, context);

  return state.parsedReqfileFromServer;
}

export function debugResetWorkspaceState(
  fileKey: string,
  context: ExtensionContext
) {
  const initState: ReqfileState = {
    env: null,
    parsedReqfileFromServer: null,
    requestExecutions: [],
  };

  context.workspaceState.update(fileKey, initState);

  return initState;
}

export const initCurrentFileState = (context: ExtensionContext) => () => {
  if (!window.activeTextEditor) {
    updateStatusText(context);
    return;
  }

  let filename = window.activeTextEditor.document.uri.toString();

  if (!filename.endsWith(".reqlang")) {
    updateStatusText(context);
    return;
  }

  getOrInitState(filename, context);

  // Default the selected environment is there's just one
  RsResult.ifOk(getParseResults(filename, context)!, (result) => {
    if (result.envs.length === 1) {
      setEnv(filename, context, result.envs[0]);
    }
  });

  updateStatusText(context);
};

export const debugResetCurrentFileState = (context: ExtensionContext) => () => {
  if (!window.activeTextEditor) {
    updateStatusText(context);
    return;
  }

  let filename = window.activeTextEditor.document.uri.toString();

  if (!filename.endsWith(".reqlang")) {
    updateStatusText(context);
    return;
  }

  context.workspaceState.update(filename, undefined);

  getOrInitState(filename, context);
};

/**
 * Get or initialize a workspace state for request file.
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @returns A newly initialized or existing workspace state for request file
 */
export function getOrInitState(
  fileKey: string,
  context: ExtensionContext
): ReqfileState {
  const state = context.workspaceState.get<ReqfileState>(fileKey);

  if (typeof state === "undefined") {
    const initState: ReqfileState = {
      env: null,
      parsedReqfileFromServer: null,
      requestExecutions: [],
    };

    context.workspaceState.update(fileKey, initState);

    return initState;
  }

  return state;
}

/**
 * Accept a function that updates the workspace state
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @param fn Function to update the state
 * @returns The newly updated workspace state
 */
export function updateState(
  fileKey: string,
  context: ExtensionContext,
  fn: (state: ReqfileState) => ReqfileState
): ReqfileState {
  const state = fn(getOrInitState(fileKey, context));
  context.workspaceState.update(fileKey, state);
  return state;
}

/**
 * Set the environment name for the request file
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @param env Environment name to set
 * @returns Updated workspace state for the request file
 */
export function setEnv(
  fileKey: string,
  context: ExtensionContext,
  env: string | null
): ReqfileState {
  return updateState(fileKey, context, (state) => {
    state.env = env;
    return state;
  });
}

export function startRequestToBeExecuted(
  fileKey: string,
  context: ExtensionContext,
  params: RequestToBeExecutedParams
) {
  updateState(fileKey, context, (state) => {
    state.requestExecutions.push({
      startDateIso: new Date().toISOString(),
      response: null,
      endDateIso: null,
      wasSuccessful: null,
      params,
    });

    return state;
  });
}

export function endRequestToBeExecuted(
  fileKey: string,
  context: ExtensionContext,
  response: HttpResponse
) {
  updateState(fileKey, context, (state) => {
    const requestToBeExecuted = state.requestExecutions.at(-1)!;

    requestToBeExecuted.response = response;
    requestToBeExecuted.endDateIso = new Date().toISOString();
    requestToBeExecuted.wasSuccessful =
      response.status_code >= 200 && response.status_code <= 299;

    state.requestExecutions[state.requestExecutions.length - 1] =
      requestToBeExecuted;

    return state;
  });
}

/**
 * Get the environment name selected for the request file
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @returns The environment name for the request file
 */
export function getEnv(
  fileKey: string,
  context: ExtensionContext
): string | null {
  const state = getOrInitState(fileKey, context);

  return state.env;
}

/**
 * Get the last received response for the request file from language server
 *
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @returns The last received response for the request file
 */
export function getLastResponse(
  fileKey: string,
  context: ExtensionContext
): RequestToBeExecuted | null {
  const state = getOrInitState(fileKey, context);
  return state.requestExecutions.at(-1) ?? null;
}

/**
 * Set the last received response for the request file from language server
 *
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @param response The last response to set
 * @returns The updated workspace state for request file
 */
export function setLastResponse(
  fileKey: string,
  context: ExtensionContext,
  response: RequestToBeExecuted
): ReqfileState {
  return updateState(fileKey, context, (state) => {
    state.requestExecutions.push(response);
    return state;
  });
}
