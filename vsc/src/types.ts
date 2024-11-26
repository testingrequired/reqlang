import { Request } from "reqlang-types";
import { Result } from "./result";

/**
 * State for an individual request file
 */
export type ReqlangWorkspaceFileState = {
  /**
   * Current selected environment
   */
  env: string | null;
  parseResult: Result<ParseResult> | null;
};

export type ParseNotification = {
  file_id: string;
  result: Result<ParseResult>;
};

export type ParseResult = {
  vars: string[];
  envs: string[];
  prompts: string[];
  secrets: string[];
  request: Request;
};

/**
 * The possible choices for the Reqlang Menu in the VSC extension
 */
export enum MenuChoices {
  PickEnv = "Pick an environment",
  RunRequest = "Run request",
}

/**
 * The possible choices for the Reqlang Menu in the VSC extension
 */
export enum Commands {
  PickEnv = "reqlang.pickEnv",
  RunRequest = "reqlang.run",
  Menu = "reqlang.menu",
  Execute = "reqlang.executeRequest",
}
