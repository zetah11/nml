import * as fs from 'fs';
import * as path from 'path';

import { ExtensionContext } from "vscode";
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
        args: ["lsp", "--channel=stdio"],
        transport: TransportKind.stdio,
    };

    const serverOptions: ServerOptions = {
        run,
        debug: run,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ "scheme": "file", "language": lang }],
    };

    client = new LanguageClient(name, serverOptions, clientOptions);
    await client.start();
}

export async function deactivate() { }

function getServerPath(): string {
    const baseDir = process.env["NML_DEBUG_DIR"];
    if (baseDir !== undefined) {
        const exe = path.join(baseDir, exec + ".exe");
        const bare = path.join(baseDir, exec);

        if (fs.existsSync(exe)) {
            return exe;
        } else if (fs.existsSync(bare)) {
            return bare;
        }
    }

    return exec;
}
