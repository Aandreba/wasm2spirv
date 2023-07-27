importScripts("highlight/highlight.min.js")

onmessage = (event) => {
    const [text, language, port] = event.data;
    const result = hljs.highlight(text, { language })
    port.postMessage(result.value)
}
