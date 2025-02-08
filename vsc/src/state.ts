"use strict";
import { ExtensionContext, window } from "vscode";
import {
  type RequestToBeExecuted,
  type ReqfileState,
  type ParsedReqfileFromServer,
} from "./types";
import * as RsResult from "rsresult";
import { updateStatusText } from "./status";
import { getClient } from "./client";

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
  result: RsResult.Result<ParsedReqfileFromServer>,
): ReqfileState {
  return updateState(fileKey, context, (state) => {
    getClient().outputChannel.appendLine(
      `Setting parsed request file for '${fileKey}': ${JSON.stringify(result)}`,
    );

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
  context: ExtensionContext,
): RsResult.Result<ParsedReqfileFromServer> | null {
  const state = getOrInitState(fileKey, context);

  return state.parsedReqfileFromServer;
}

export function debugResetWorkspaceState(
  fileKey: string,
  context: ExtensionContext,
) {
  const initState: ReqfileState = {
    env: null,
    parsedReqfileFromServer: null,
    isWaitingForResponse: false,
    requestExecutions: [],
  };

  context.workspaceState.update(fileKey, initState);

  return initState;
}

export const initCurrentFileState = (context: ExtensionContext) => {
  if (!window.activeTextEditor) {
    updateStatusText(context);
    return;
  }

  const filename = window.activeTextEditor.document.uri.toString();

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

  const filename = window.activeTextEditor.document.uri.toString();

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
  context: ExtensionContext,
): ReqfileState {
  const state = context.workspaceState.get<ReqfileState>(fileKey);

  if (typeof state === "undefined") {
    const initState: ReqfileState = {
      env: null,
      parsedReqfileFromServer: null,
      isWaitingForResponse: false,
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
  fn: (state: ReqfileState) => ReqfileState,
): ReqfileState {
  const state = fn(getOrInitState(fileKey, context));
  context.workspaceState.update(fileKey, state);
  updateStatusText(context);
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
  env: string | null,
): ReqfileState {
  return updateState(fileKey, context, (state) => {
    state.env = env;
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
  context: ExtensionContext,
): string | null {
  const state = getOrInitState(fileKey, context);

  return state.env;
}

/**
 * Get if a request file is waiting an execute request response from language
 * server
 *
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @returns If the request file is waiting for a response
 */
export function getIsWaitingForResponse(
  fileKey: string,
  context: ExtensionContext,
): boolean {
  const state = getOrInitState(fileKey, context);

  return state.isWaitingForResponse;
}

/**
 * Set if a request file is waiting an execute request response from language
 * server
 *
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @param isWaitingForResponse Value to set
 * @returns Updated workspace state for request file
 */
export function setIsWaitingForResponse(
  fileKey: string,
  context: ExtensionContext,
  isWaitingForResponse: boolean,
): ReqfileState {
  return updateState(fileKey, context, (state) => {
    state.isWaitingForResponse = isWaitingForResponse;
    return state;
  });
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
  context: ExtensionContext,
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
  response: RequestToBeExecuted,
): ReqfileState {
  return updateState(fileKey, context, (state) => {
    state.requestExecutions.push(response);
    return state;
  });
}
