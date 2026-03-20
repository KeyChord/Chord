// This code only supports `require` with built-in modules; not from file-system.
{% for module in builtinModules %}
import {{ module }} from '{{ module }}'
{% endfor %}

globalThis.require = function require(filepath) {
  {% for module in builtinModules %}
  if (filepath === '{{ module }}') {
    return {{ module }}
  }
  {% endfor %}
}

globalThis.createRequire = function createRequire(filepath) {
  return require
}