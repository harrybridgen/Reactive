import * as vscode from "vscode";

export function activate(context: vscode.ExtensionContext) {
  const formatter = vscode.languages.registerDocumentFormattingEditProvider(
    "reactive",
    {
      provideDocumentFormattingEdits(
  document: vscode.TextDocument
): vscode.TextEdit[] {
  const text = document.getText();
  const rawLines = text.split(/\r?\n/);

  // Step 1: Attach standalone opening braces to previous line
  const mergedLines: string[] = [];
  for (const rawLine of rawLines) {
    const line = rawLine.trim();

    if (line === "{" && mergedLines.length > 0) {
      // Attach brace to previous line
      mergedLines[mergedLines.length - 1] =
        mergedLines[mergedLines.length - 1].replace(/\s*$/, "") + " {";
    } else {
      mergedLines.push(rawLine);
    }
  }

  // Step 2: Indentation pass
  let indentLevel = 0;
  const indentUnit = " ".repeat(4);

  const formattedLines = mergedLines.map((rawLine) => {
    const line = rawLine.trim();

    if (line.length === 0) {
      return "";
    }

    if (line.startsWith("}")) {
      indentLevel = Math.max(indentLevel - 1, 0);
    }

    const indentedLine = indentUnit.repeat(indentLevel) + line;

    if (line.endsWith("{")) {
      indentLevel++;
    }

    return indentedLine;
  });

  const formatted = formattedLines.join("\n");

  const fullRange = new vscode.Range(
    document.positionAt(0),
    document.positionAt(text.length)
  );

  return [vscode.TextEdit.replace(fullRange, formatted)];
}

    }
  );

  context.subscriptions.push(formatter);
}
