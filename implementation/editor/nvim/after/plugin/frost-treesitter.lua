local ok, parsers = pcall(require, "nvim-treesitter.parsers")
if not ok then
  return
end

local parser_configs = parsers.get_parser_configs()
if parser_configs.frost ~= nil then
  return
end

local script_path = debug.getinfo(1, "S").source:sub(2)
local editor_dir = vim.fn.fnamemodify(script_path, ":p:h:h:h:h")
local grammar_dir = editor_dir .. "/tree-sitter-frost"

parser_configs.frost = {
  install_info = {
    url = grammar_dir,
    files = { "src/parser.c" },
    generate_requires_npm = false,
    requires_generate_from_grammar = false,
  },
  filetype = "frost",
}
