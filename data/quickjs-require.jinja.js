// This code only supports `require` with built-in modules; not from file-system.
{%- for module in builtinModules %}
import * as __require_{{ loop.index }} from '{{ module }}';
{%- endfor %}

function require(filepath) {
  {%- for module in builtinModules %}
  if (filepath === '{{ module }}') return __require_{{ loop.index }}.default;
  {%- endfor %}
}

globalThis.require = require;
globalThis.createRequire = function createRequire(filepath) {
  return require
}
