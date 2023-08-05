import * as fs from 'fs';
import * as path from 'path';

import { ExtensionContext, window } from "vscode";
import { LanguageClientOptions, LanguageClient, ServerOptions, Executable, TransportKind } from "vscode-languageclient/node";

const exec = "nmlc";
const lang = "nml";
const name = "nml";

let client: LanguageClient;

export async function activate(_context: ExtensionContext) {
    const server = getServerPath();
    console.log(server);
    const run: Executable = {
        command: server,
        args: ["lsp", "--log", "trace"],
        transport: TransportKind.stdio,
    };

    const serverOptions: ServerOptions = {
        run,
        debug: run,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ "scheme": "file", "language": lang }],
        traceOutputChannel: window.createOutputChannel(`${name} trace`, lang),
    };

    client = new LanguageClient(name, serverOptions, clientOptions);
    await client.start();
}

export async function deactivate() { }

function getServerPath(): string {
    const baseDir = process.env["__NML_DEBUG_EXTENSION_DIR"];

    if (baseDir !== undefined) {
        const ext = process.platform === "win32" ? ".exe" : "";
        const exe = path.join(baseDir, exec + ext);

        if (fs.existsSync(exe)) {
            return exe;
        }
    }

    return exec;
}
