import { open, FileHandle } from "node:fs/promises"
const GRAMMAR_URL = "https://raw.githubusercontent.com/KhronosGroup/SPIRV-Headers/main/include/spirv/unified1/spirv.core.grammar.json"

async function translate() {
    const [payload, outputFile] = await Promise.all([loadPayload(GRAMMAR_URL), open("spirv.js", "w")])

    try {
        for (let operand of payload.operand_kinds) {
            switch (operand.category) {
                case "ValueEnum":
                    await translateEnum(operand, outputFile);
                    break;
                case "BitEnum":
                    await translateBitField(operand, outputFile);
                    break;
                default:
                    console.warn(`Unknown value category: ${operand.category}`)
                    continue;
            }
        }
    } finally {
        await outputFile.close()
    }
}

// ValueEnum
async function translateEnum(value: any, outputFile: FileHandle) {
    let typedef: string[] = []
    let enumerants = new Map<string, number>()

    for (let enumerant of value.enumerants) {
        typedef.push(`"${enumerant.enumerant}"`)
        enumerants.set(enumerant.enumerant, enumerant.value)
    }
    await outputFile.write(`/** @typedef {(${typedef.join('|')})} ${value.kind} */\n`);

    let intToStr = ""
    let strToInt = ""

    for (let [key, value] of enumerants.entries()) {
        strToInt += `\tcase "${key}":\n\t\treturn ${value};\n`;
        intToStr += `\tcase ${value}:\n\t\treturn "${key}";\n`;
    }

    await outputFile.write(
        `/** \n * @param {${value.kind}} value\n * @returns {number} */\nexport function integer${value.kind}(value) {\n\tswitch(value) {\n`
    )
    await outputFile.write(strToInt);
    await outputFile.write("\tdefault:\n\t\tthrow new Error(\"Unexpected value\");\n\t}\n}\n")

    await outputFile.write(
        `/** \n * @param {number} value\n * @returns {${value.kind}} */\nexport function string${value.kind}(value) {\n\tswitch(value) {\n`
    )
    await outputFile.write(intToStr);
    await outputFile.write("\tdefault:\n\t\tthrow new Error(\"Unexpected value\");\n\t}\n}\n")
}

// BitEnum
async function translateBitField(value: any, outputFile: FileHandle) {
    await outputFile.write(`/** @typedef {number} ${value.kind} */\n`);

    let namespace = to_snake_case(value.kind, "upper")
    for (let enumerant of value.enumerants) {
        let enumerantName = to_snake_case(enumerant.enumerant, "upper");
        await outputFile.write(
            `/** @type {${value.kind}} */\nexport const ${namespace}_${enumerantName} = ${enumerant.value};\n`
        )
    };
}

/* UTILS */
function to_snake_case(camelCase: string, textcase?: "upper" | "lower") {
    let result = ""
    const caseF = textcase === "upper" ? toAsciiUppercase : (textcase === "lower" ? toAsciiLowercase : (x: number) => x)

    for (let i = 0; i < camelCase.length; i++) {
        let ch = camelCase.charCodeAt(i);

        if (result.length > 0 && isAsciiUppercase(ch) && (isAsciiLowercase(camelCase.charCodeAt(i - 1)) || isAsciiLowercase(camelCase.charCodeAt(i + 1)))) {
            result += `_${String.fromCharCode(caseF(ch))}`
        } else {
            result += String.fromCharCode((caseF)(ch))
        }
    }

    return result
}

async function loadPayload(url: string) {
    const response = await fetch(url)
    return await response.json();
}

/* UTILS */
function parameterKindToType(kind: string): string | undefined {
    switch (kind) {
        case "LiteralInteger":
            return 'number';
        case "LiteralString":
            return 'string';
        default:
            return undefined
    }
}

/* ASCII */
function isAsciiUppercase(char: number): boolean {
    return char >= 65 && char <= 90
}

function isAsciiLowercase(char: number): boolean {
    return char >= 97 && char <= 122
}

function toAsciiUppercase(char: number): number {
    return isAsciiLowercase(char) ? asciiChangeCase(char) : char
}

function toAsciiLowercase(char: number): number {
    return isAsciiUppercase(char) ? asciiChangeCase(char) : char
}

function asciiChangeCase(char: number): number {
    return char ^ 0b0010_0000
}

translate()
