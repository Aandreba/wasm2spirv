// in milliseconds
const THROTTLE_INTERVAL = 1000;

const sourceEditor = document.getElementById("source")
const watEditor = document.querySelector("#result #wat")
const spvEditor = document.querySelector("#result #spv")

const functionIdx = document.getElementById("index")
const executionModel = document.getElementById("execution_model")
const language = document.getElementById("lang")

sourceEditor.addEventListener("keyup", function() {
    highlight(sourceEditor, language.value)
    update()
})

functionIdx.addEventListener("change", update);
executionModel.addEventListener("change", update);
language.addEventListener("change", update);

let abortController = null
async function update() {
    if (abortController) abortController.abort()
    let controller = new AbortController()
    let signal = controller.signal;

    setTimeout(() => {
        if (signal.aborted) return;
        compute(signal);
    }, THROTTLE_INTERVAL)

    abortController = controller
}

async function compute(signal) {
    watEditor.style.opacity = 0.5
    spvEditor.style.opacity = 0.5

    const response = await fetch("/api/compile", {
        method: "post",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(buildBody()),
        signal
    })

    watEditor.style.opacity = 1
    spvEditor.style.opacity = 1

    if (!response.ok) {
        watEditor.style.color = "red"
        watEditor.innerHTML = removeAnsi(await response.text())
        return;
    }

    const body = await response.json();
    watEditor.innerHTML = body.wat;

    if ("Ok" in body.spv) {
        spvEditor.style.color = "white"
        spvEditor.innerHTML = body.spv.Ok
    } else if ("Err" in body.spv) {
        spvEditor.style.color = "red"
        spvEditor.innerHTML = body.spv.Err
    } else {
        // TODO
        console.error(body.spv)
    }

    highlight(watEditor, "wasm");
}

function buildBody() {
    let object = {
        lang: language.value,
        source: sourceEditor.value,
        functions: {}
    }

    object.functions[functionIdx.value] = {
        execution_model: executionModel.value,
        execution_modes: [{
            "local_size": [1, 1, 1]
        }],
        params: {}
    };

    return object
}

const highlightWorker = new Worker("highlight.js")
function highlight(codearea, language) {
    const channel = new MessageChannel()
    highlightWorker.postMessage([codearea.innerHTML, language], [channel.port2])

    channel.port1.start()
    channel.port1.addEventListener(
        "message",
        event => codearea.innerHTML = event.data,
        { once: true }
    )
}

function removeAnsi(s) {
    return s.replace(/[\u001b\u009b][[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g, "");
}

window.addEventListener("load", function() {
    update();
}, {
    once: true
})
