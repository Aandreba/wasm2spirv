# Zig syntax support for [highlight.js](https://highlightjs.org/)

Syntax highlighting for Zig using highlight.js

## Usage

Include the Highlight.js library in your webpage or Node app, then load this module.

### Static website or simple usage

Simply load the module after loading highlight.js. You'll use the minified version found in the `dist` directory. This module is just a CDN build of the language, so it will register itself as the Javascript is loaded.

```html
<script type="text/javascript" src="/path/to/highlight.min.js"></script>
<script
  type="text/javascript"
  src="/path/to/highlightjs-zig/dist/zig.min.js"
></script>
<script type="text/javascript">
  hljs.initHighlightingOnLoad();
</script>
```

This will find and highlight code inside of `<pre><code>` tags; it tries to detect the language automatically. If automatic detection doesn’t work for you, you can specify the language in the `class` attribute:

```html
<pre>
    <code class="zig">
    ...
    </code>
</pre>
```

### With Node or another build system

If you're using Node / Webpack / Rollup / Browserify, etc, simply require the language module, then register it with Highlight.js.

```javascript
var hljs = require("highlightjs");
var hljsZig = require("highlightjs-zig");

hljs.registerLanguage("zig", hljsZig);
hljs.initHighlightingOnLoad();
```

## License

highlightjs-zig is released under the MIT License. See [LICENSE](https://notabug.org/Ash/highlightjs-zig/src/master/LICENSE) file for details.

## Author

Ash Ametrine <ash@kinzie.dev>

## Links

- The official site for the Highlight.js library is <https://highlightjs.org/>.
- The Highlight.js GitHub project: <https://github.com/highlightjs/highlight.js>
- Learn more about Zig: <https://ziglang.org/>
