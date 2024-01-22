import * as vscode from "vscode";

export class ReqlangTaskProvider implements vscode.TaskProvider {
  static TaskType = "reqlang";
  private task: vscode.Task | undefined;

  constructor() {}

  async provideTasks(token: vscode.CancellationToken): Promise<vscode.Task[]> {
    const tasks = [];
    return tasks;
  }

  resolveTask(
    task: vscode.Task,
    _token: vscode.CancellationToken
  ): vscode.Task | undefined {
    return undefined;
  }

  private getCurrentOpenFilePath(): string {
    return vscode.window.activeTextEditor.document.uri.path;
  }
}
