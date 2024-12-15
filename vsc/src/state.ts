"use strict";
import { ExtensionContext } from "vscode";
import type {
  RecordedHttpResponse,
  ReqlangWorkspaceFileState,
  SimplifiedParsedRequestFile,
} from "./types";
import * as RsResult from "rsresult";
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
  result: RsResult.Result<SimplifiedParsedRequestFile>
): ReqlangWorkspaceFileState {
  return updateState(fileKey, context, (state) => {
    state.parsedReqfile = result;
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
): RsResult.Result<SimplifiedParsedRequestFile> | null {
  const state = getOrInitState(fileKey, context);

  return state.parsedReqfile;
}

export function debugResetWorkspaceState(
  fileKey: string,
  context: ExtensionContext
) {
  const initState: ReqlangWorkspaceFileState = {
    env: null,
    parsedReqfile: null,
    isWaitingForResponse: false,
    responses: [],
  };

  context.workspaceState.update(fileKey, initState);

  return initState;
}

/**
 * Get or initialize a workspace state for request file.
 * @param fileKey File uri of the request file
 * @param context Extension context from VS Code
 * @returns A newly initialized or existing workspace state for request file
 */
export function getOrInitState(
  fileKey: string,
  context: ExtensionContext
): ReqlangWorkspaceFileState {
  const state = context.workspaceState.get<ReqlangWorkspaceFileState>(fileKey);

  if (typeof state === "undefined") {
    const initState: ReqlangWorkspaceFileState = {
      env: null,
      parsedReqfile: null,
      isWaitingForResponse: false,
      responses: [],
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
  fn: (state: ReqlangWorkspaceFileState) => ReqlangWorkspaceFileState
): ReqlangWorkspaceFileState {
  const state = fn(getOrInitState(fileKey, context));
  context.workspaceState.update(fileKey, state);
  console.log(`NEW STATE: ${JSON.stringify(state)}`);
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
): ReqlangWorkspaceFileState {
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
  context: ExtensionContext
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
  context: ExtensionContext
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
  isWaitingForResponse: boolean
): ReqlangWorkspaceFileState {
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
  context: ExtensionContext
): RecordedHttpResponse | null {
  const state = getOrInitState(fileKey, context);

  return state.responses[state.responses.length - 1] ?? null;
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
  response: RecordedHttpResponse
): ReqlangWorkspaceFileState {
  return updateState(fileKey, context, (state) => {
    state.responses.push(response);
    return state;
  });
}
