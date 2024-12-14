import {
  CancellationToken,
  CodeLens,
  CodeLensProvider,
  ExtensionContext,
  ProviderResult,
  Range,
  TextDocument,
} from "vscode";
import {
  getEnv,
  getIsWaitingForResponse,
  getParseResults,
  setIsWaitingForResponse,
} from "./state";
import { expect } from "rsresult";
import { Commands } from "./types";

/**
 * A codelens provider for request files
 */
export class ReqlangCodeLensProvider implements CodeLensProvider {
  constructor(private context: ExtensionContext) {}

  /**
   * Provides code lenses for choosing environments and running requests.
   */
  provideCodeLenses(
    document: TextDocument,
    _token: CancellationToken
  ): ProviderResult<CodeLens[]> {
    const lenses = [];

    /**
     * Used to access this request file's workspace state.
     *
     * Prepending the uri with `file://` is necessary because vscode doesn't automatically resolve relative paths.
     * This also matches how the workspace state per file keys are set.
     */
    const uri = `file://${document.fileName}`;

    /**
     * The full (not simplified) parsed request file.
     *
     * This is used to get the request's span in the source text
     *
     * The span is used to calculate the lens's postition
     */
    const { full: reqFile } = expect(
      getParseResults(uri, this.context)!,
      "There should be a parsed request file in the workspace state. This is sent by the language server to the client."
    );

    /**
     * Get the request span from the parsed reqfile.
     * This will be used to position the lens above the request in the request file.
     */
    const [_, requestSpan] = reqFile.request;

    const lensRange = new Range(
      document.positionAt(requestSpan.start),
      document.positionAt(requestSpan.end)
    );

    const isWaitingForResponse = getIsWaitingForResponse(uri, this.context);

    /**
     * The request file's selected environment from the workspace state.
     * This might be null if the user hasn't selected an environment.
     */
    const env = getEnv(uri, this.context);

    // If an environment is set, add a run request lens
    if (env !== null) {
      lenses.push(
        new CodeLens(lensRange, {
          command: Commands.RunRequest,
          title: isWaitingForResponse ? "$(pause) Running" : `$(run) Run`,
        })
      );
    }

    const numberOfEnvs = Object.keys(reqFile.config![0].envs ?? {}).length;

    if (numberOfEnvs > 1) {
      // Add a pick environment lens
      lenses.push(
        new CodeLens(lensRange, {
          command: Commands.PickEnv,
          title: `$(globe) ${env ? env : "Env..."}`,
        })
      );
    }

    return lenses;
  }
}
