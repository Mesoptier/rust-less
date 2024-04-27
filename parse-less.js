const fs = require('node:fs/promises');
const process = require('node:process');
const readline = require('node:readline');
const {parseArgs} = require('node:util');

const less = require('less');

const {values: args} = parseArgs({
    options: {
        file: {
            type: 'string',
        },
        stdin: {
            type: 'boolean',
        },
    },
});

if (!args.file && !args.stdin) {
    console.error('Usage: parse-less.js --file <file> | --stdin');
    process.exit(1);
}

async function getLessSource() {
    if (args.file) {
        return await fs.readFile(args.file, 'utf-8');
    }
    if (args.stdin) {
        const rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout,
        });

        let lessSource = '';
        for await (const line of rl) {
            lessSource += line + '\n';
        }
        return lessSource;
    }
    process.exit(1);
}

getLessSource()
    .then((lessSource) => {
        return less.parse(lessSource);
    })
    .then((node) => {
        process.stdout.write(JSON.stringify(toJSON(node, null), null, 2));

        process.exit(0);
    });

function toJSON(node, parent) {
    if (typeof node !== 'object' || node === null || !('type' in node)) {
        return node;
    }

    if (node instanceof less.tree.Declaration) {
        parent.parseValue(node);
    }

    const json = {
        type: node.type,
    };

    Object.entries(node).forEach(([key, value]) => {
        if (key.startsWith('_')) {
            return;
        }
        if (['parent', 'allowRoot', 'functionRegistry', 'parsed', 'strictImports'].includes(key)) {
            return;
        }
        if (typeof value === 'function') {
            return;
        }
        if (Array.isArray(value)) {
            json[key] = value.map((child) => toJSON(child, node));
            return;
        }
        json[key] = toJSON(value, node);
    });

    return json;
}
