# Reactive Language Support for VS Code

VS Code language support for the Reactive programming language.
This extension provides:

- Syntax highlighting for .rx files
- Automatic code formatting (indentation + brace style)

## Features

### Syntax Highlighting

- Keywords, types, functions, operators, and bindings
- Block comments

### Formatting

- 4-space indentation
- Block-based indentation using { and }
- Opening braces are placed on the same line

Example:

```lua
func main()
{
print(1);
}
```

Formats to:

```lua
func main() {
    print(1);
}
```

## Installation (Manual)

This extension is currently installed manually via a VSIX package.

1. Open the Extensions view in VS Code
2. Click the ⋯ menu
3. Select Install from VSIX…
4. Choose the generated `.vsix` file (for example: `rx-vscode-0.0.1.vsix`)

Syntax highlighting should activate automatically for .rx files.

### Packaging the Extension Yourself

From the rx-vscode directory:

```
npm install
npx tsc
vsce package
```

This will generate a `.vsix` file which can be installed using Install from VSIX….

### Formatter Setup (Important)

To enable formatting for Reactive files, you should set the formatter at the workspace level.

Workspace Settings (Recommended)

In your project workspace, create or edit:

```
.vscode/settings.json
```

Add the following:

```
{
  "[reactive]": {
    "editor.defaultFormatter": "local.rx-vscode"
  }
}
```

This ensures:

- The Reactive formatter is used automatically
- VS Code does not prompt you to select a formatter each time

| Note: The extension ID may differ if you change the publisher name.
| You can copy the exact ID from the Extensions view if needed.

## File Association (Optional)

If VS Code does not automatically associate .rx files, add this to your settings:

```
"files.associations": {
  "*.rx": "reactive"
}
```
