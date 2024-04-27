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
        return less.parse(lessSource, {
            processImports: false,
        });
    })
    .then((node) => {
        const json = toJSON(node);
        process.stdout.write(JSON.stringify(json, null, 2));
        process.exit(0);
    });

function toJSON(node) {
    if (Array.isArray(node)) {
        return node.map((child) => toJSON(child));
    }
    if (typeof node !== 'object' || node === null) {
        return node;
    }
    if (!(node instanceof less.tree.Node)) {
        console.error(`Unexpected object`, node);
        throw new Error('Unexpected object');
    }

    if (node.type === 'Expression' && node.parens) {
        return toJSON(node.value[0]);
    }
    if (node.type === 'Declaration') {
        if (node.parent?.type === 'Ruleset') {
            node.parent.parseValue(node);
        }
    }

    const json = {
        type: node.type,
    };

    Object.entries(node).forEach(([key, value]) => {
        if (key.startsWith('_')) {
            return;
        }
        const keyBlacklist = [
            'parent',
            'allowRoot',
            'functionRegistry',
            'parsed',
            'strictImports',
            'isSpaced',
            'variableRegex',
            'propRegex',
            'quote',
        ];
        if (keyBlacklist.includes(key)) {
            return;
        }

        if (node.type === 'MixinDefinition') {
            if (key === 'params') {
                json[key] = value.map((param) => ({
                    ...param,
                    value: toJSON(param.value),
                }));
                return;
            }
        }

        if (node.type === 'MixinCall') {
            if (key === 'arguments') {
                json[key] = value.map((param) => ({
                    ...param,
                    value: toJSON(param.value),
                }));
                return;
            }
        }

        if (node.type === 'Import') {
            if (key === 'options') {
                json[key] = value;
                return;
            }
        }

        if (typeof value === 'function') {
            return;
        }
        json[key] = toJSON(value);
    });

    return json;
}
