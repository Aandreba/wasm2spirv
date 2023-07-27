importScripts("highlight/highlight.min.js")

onmessage = (event) => {
    const [text, language] = event.data;
    const result = hljs.highlight(text, { language })
    event.ports[0].postMessage(result.value)
}
