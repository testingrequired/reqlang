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
  getLastResponse,
  getParseResults,
} from "./state";
import { expect } from "rsresult";
import { Commands, RecordedHttpResponse } from "./types";
import { UnresolvedRequestFile } from "reqlang-types";
import { formatDistance } from "date-fns";

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

    // Add menu codelens at the top of the request file
    lenses.push(new MenuCodeLens(new Range(0, 0, 0, 0)));

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

    const requestLensRange = new Range(
      document.positionAt(requestSpan.start),
      document.positionAt(requestSpan.end)
    );

    /**
     * The last response, if it exists
     */
    const lastResponse = getLastResponse(uri, this.context);

    /**
     * The request file's selected environment from the workspace state.
     * This might be null if the user hasn't selected an environment.
     */
    const env = getEnv(uri, this.context);

    // If an environment is set, add a run request lens
    if (env !== null) {
      lenses.push(
        new RunRequestCodeLens(
          requestLensRange,
          getIsWaitingForResponse(uri, this.context)
        )
      );
    }

    if (lastResponse !== null) {
      lenses.push(new LastReponseCodeLens(requestLensRange, lastResponse));
    }

    // If there are more than one environment in the request file, add a pick environment lens
    if (getEnvsFromReqfile(reqFile).length > 1) {
      lenses.push(new EnvPickerCodeLens(requestLensRange, env));
    }

    return lenses;
  }
}

/**
 * A codelens to show the reqlang menu
 */
class MenuCodeLens extends CodeLens {
  constructor(requestLensRange: Range) {
    super(requestLensRange, {
      title: "$(menu)",
      tooltip: "Open the reqlang menu",
      command: Commands.Menu,
    });
  }
}

/**
 * A codelens to run the request in the reqfile.
 */
class RunRequestCodeLens extends CodeLens {
  // Constructor that takes in a bool if we are waiting for a response
  constructor(requestLensRange: Range, isWaitingForResponse: boolean) {
    super(requestLensRange, {
      command: Commands.RunRequest,
      title: isWaitingForResponse ? "$(pause) Running" : "$(run) Run",
    });
  }
}

/**
 * A codelens to display the last resposne.
 */
class LastReponseCodeLens extends CodeLens {
  constructor(requestLensRange: Range, lastResponse: RecordedHttpResponse) {
    const ago = formatDistance(new Date(), lastResponse.recieved);
    const icon = lastResponse.wasSuccessful ? "check" : "error";

    super(requestLensRange, {
      command: Commands.ShowResponse,
      title: `$(${icon}) (${ago} ago)`,
      arguments: [lastResponse.response],
    });
  }
}

/**
 * A codelens to pick an environment from the request file.
 */
class EnvPickerCodeLens extends CodeLens {
  constructor(range: Range, env: string | null) {
    super(range, {
      command: Commands.RunRequest,
      title: `$(globe) ${env ? env : "Env..."}`,
    });
  }
}

/**
 * Get environment names from a request file.
 * @param reqFile The request file to get environments from
 * @returns Array of environment names
 */
function getEnvsFromReqfile(reqFile: UnresolvedRequestFile) {
  return Object.keys(reqFile.config?.[0]?.envs ?? {});
}
