// in milliseconds
const THROTTLE_INTERVAL = 1000
const red = "#DC143C"

const sourceEditor = document.getElementById("source")
const configEditor = document.getElementById("config")

const watEditor = document.querySelector("#result #wat")
const resultEditor = document.querySelector("#result #spv")

const language = document.getElementById("lang")
const compilationLanguage = document.getElementById("compile-lang")
const optimization = document.getElementById("optimization")

sourceEditor.addEventListener("keyup", update)
configEditor.addEventListener("keyup", update)
language.addEventListener("change", update)
compilationLanguage.addEventListener("change", update)
optimization.addEventListener("change", update)

let abortController = null
async function update() {
    if (abortController) abortController.abort()
    let controller = new AbortController()
    let signal = controller.signal

    setTimeout(() => {
        if (signal.aborted) return
        compute(signal)
    }, THROTTLE_INTERVAL)

    abortController = controller
}

async function compute(signal) {
    watEditor.style.opacity = 0.5
    resultEditor.style.opacity = 0.5

    const body = buildBody()
    const response = await fetch("/api/compile", {
        method: "post",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(body),
        signal
    })

    if (!response.ok) {
        resultEditor.style.color = red
        highlight(await response.text(), resultEditor, undefined)
        watEditor.innerHTML = ""
        return
    }
    const payload = await response.json()

    if ("Ok" in payload.result) {
        resultEditor.style.color = "white"
        const highlighLang = highlighLanguage(body.compile_lang)
        highlight(payload.result.Ok, resultEditor, highlighLang)
    } else if ("Err" in payload.result) {
        resultEditor.style.color = red
        resultEditor.innerHTML = payload.result.Err
    } else {
        // TODO
        console.error(payload.result)
    }

    highlight(payload.wat, watEditor, "wasm")
}

function buildBody() {
    let config;
    try {
        config = JSON.parse(configEditor.value)
    } catch {
        config = {}
    };

    let object = {
        lang: language.value,
        compile_lang: compilationLanguage.value,
        source: sourceEditor.value,
        config,
        optimization_runs: parseInt(optimization.value)
    }

    return object
}

const highlightWorker = new Worker("highlight.js")
function highlight(code, codearea, language) {
    if (language) {
        const channel = new MessageChannel()
        highlightWorker.postMessage([code, language], [channel.port2])

        channel.port1.start()
        channel.port1.addEventListener(
            "message",
            event => {
                codearea.innerHTML = event.data;
                codearea.style.opacity = 1;
            },
            { once: true }
        )
    } else {
        codearea.innerHTML = code;
        codearea.style.opacity = 1;
    }
}

function highlighLanguage(compilationLanguage) {
    switch (compilationLanguage) {
        case "glsl":
            return compilationLanguage
        case "msl":
            return "cpp"
        default:
            return undefined
    }
}

function removeAnsi(s) {
    return s.replace(/[\u001b\u009b][[()#?]*(?:[0-9]{1,4}(?:[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g, "")
}

window.addEventListener("load", function () {

    const configEditor = this.document.getElementById("config-editor");
    setupEditor(configEditor, () => "json");

    const mainEditor = this.document.getElementById("main-editor");
    const updater = setupEditor(mainEditor, () => language.value);
    language.addEventListener("change", updater)

    update();
}, {
    capture: true,
    once: true
})

function setupEditor(editor, lang) {
    const textarea = editor.querySelector("textarea")
    const code = editor.querySelector("code")

    textarea.o

    textarea.addEventListener("scroll", () => {
        code.scrollTop = textarea.scrollTop;
        code.scrollLeft = textarea.scrollLeft;
    })

    const updater = () => {
        const language = (lang)()
        code.innerHTML = hljs.highlight(textarea.value, { language }).value;
    };

    textarea.addEventListener("keyup", () => {
        const trimmed = textarea.value.trim();
        if (textarea.value != trimmed)
            textarea.value = trimmed
        updater();
    })

    textarea.addEventListener("resize", updater)
    return updater
}
