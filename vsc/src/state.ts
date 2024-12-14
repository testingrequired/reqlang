"use strict";
import { ExtensionContext } from "vscode";
import type {
  ReqlangWorkspaceFileState,
  SimplifiedParsedRequestFile,
} from "./types";
import * as RsResult from "rsresult";

export function setParseResult(
  fileKey: string,
  context: ExtensionContext,
  result: RsResult.Result<SimplifiedParsedRequestFile>
): ReqlangWorkspaceFileState {
  const state = initState(fileKey, context);

  state.parsedReqfile = result;

  context.workspaceState.update(fileKey, state);

  return state;
}

export function getParseResults(
  fileKey: string,
  context: ExtensionContext
): RsResult.Result<SimplifiedParsedRequestFile> | null {
  const state = initState(fileKey, context);

  return state.parsedReqfile;
}

export function initState(
  fileKey: string,
  context: ExtensionContext
): ReqlangWorkspaceFileState {
  const state = context.workspaceState.get<ReqlangWorkspaceFileState>(fileKey);

  if (typeof state === "undefined") {
    const initState: ReqlangWorkspaceFileState = {
      env: null,
      parsedReqfile: null,
      isWaitingForResponse: false,
    };

    context.workspaceState.update(fileKey, initState);

    return initState;
  }

  return state;
}

export function setEnv(
  fileKey: string,
  context: ExtensionContext,
  env: string | null
): ReqlangWorkspaceFileState {
  const state = initState(fileKey, context);

  state.env = env;

  context.workspaceState.update(fileKey, state);

  return state;
}

export function getEnv(
  fileKey: string,
  context: ExtensionContext
): string | null {
  const state = initState(fileKey, context);

  return state.env;
}

// Get isWaitingForResponse in state
export function getIsWaitingForResponse(
  fileKey: string,
  context: ExtensionContext
): boolean {
  const state = initState(fileKey, context);

  return state.isWaitingForResponse;
}

export function setIsWaitingForResponse(
  fileKey: string,
  context: ExtensionContext,
  isWaitingForResponse: boolean
): ReqlangWorkspaceFileState {
  const state = initState(fileKey, context);

  state.isWaitingForResponse = isWaitingForResponse;

  context.workspaceState.update(fileKey, state);

  return state;
}
