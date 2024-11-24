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
