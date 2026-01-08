import * as vscode from 'vscode';
import * as path from 'path';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('rustor');

    if (!config.get<boolean>('enable', true)) {
        return;
    }

    // Start the language server
    await startLanguageServer(context);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('rustor.restart', async () => {
            await restartLanguageServer(context);
        }),
        vscode.commands.registerCommand('rustor.fixFile', async () => {
            await fixCurrentFile();
        }),
        vscode.commands.registerCommand('rustor.fixWorkspace', async () => {
            await fixWorkspace();
        })
    );

    // Watch for configuration changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(async (e) => {
            if (e.affectsConfiguration('rustor')) {
                await restartLanguageServer(context);
            }
        })
    );
}

async function startLanguageServer(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('rustor');
    const rustorPath = config.get<string>('path', 'rustor');
    const phpVersion = config.get<string>('phpVersion', '8.2');
    const preset = config.get<string>('preset', 'recommended');

    // Server options - run rustor with --lsp flag
    const serverOptions: ServerOptions = {
        command: rustorPath,
        args: ['--lsp', '--php-version', phpVersion, '--preset', preset],
        transport: TransportKind.stdio,
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'php' }
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.php')
        },
        outputChannelName: 'Rustor',
        traceOutputChannel: vscode.window.createOutputChannel('Rustor Trace'),
    };

    // Create and start the client
    client = new LanguageClient(
        'rustor',
        'Rustor Language Server',
        serverOptions,
        clientOptions
    );

    try {
        await client.start();
        vscode.window.showInformationMessage('Rustor language server started');
    } catch (error) {
        vscode.window.showErrorMessage(
            `Failed to start Rustor language server: ${error}. ` +
            `Make sure 'rustor' is installed and in your PATH, or configure 'rustor.path'.`
        );
    }
}

async function restartLanguageServer(context: vscode.ExtensionContext) {
    if (client) {
        await client.stop();
        client = undefined;
    }
    await startLanguageServer(context);
}

async function fixCurrentFile() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'php') {
        vscode.window.showWarningMessage('No PHP file is currently active');
        return;
    }

    const config = vscode.workspace.getConfiguration('rustor');
    const rustorPath = config.get<string>('path', 'rustor');
    const filePath = editor.document.uri.fsPath;

    // Save the file first
    await editor.document.save();

    // Run rustor --fix on the file
    const terminal = vscode.window.createTerminal('Rustor');
    terminal.show();
    terminal.sendText(`${rustorPath} "${filePath}" --fix`);
}

async function fixWorkspace() {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        vscode.window.showWarningMessage('No workspace folder is open');
        return;
    }

    const config = vscode.workspace.getConfiguration('rustor');
    const rustorPath = config.get<string>('path', 'rustor');

    // Confirm with user
    const result = await vscode.window.showWarningMessage(
        'This will modify PHP files in your workspace. Continue?',
        'Yes',
        'No'
    );

    if (result !== 'Yes') {
        return;
    }

    // Run rustor --fix on the workspace
    const terminal = vscode.window.createTerminal('Rustor');
    terminal.show();
    for (const folder of workspaceFolders) {
        terminal.sendText(`${rustorPath} "${folder.uri.fsPath}" --fix`);
    }
}

export async function deactivate() {
    if (client) {
        await client.stop();
    }
}
