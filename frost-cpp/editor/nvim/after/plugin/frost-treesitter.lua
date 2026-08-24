local function parser_configs()
  local ok, parsers = pcall(require, "nvim-treesitter.parsers")
  if not ok then
    return nil
  end

  if type(parsers.get_parser_configs) == "function" then
    -- Older nvim-treesitter API.
    return parsers.get_parser_configs()
  end
  if type(parsers) == "table" then
    -- Newer nvim-treesitter API exports the parser config table directly.
    return parsers
  end
  return nil
end

local function grammar_dir()
  local source_path = debug.getinfo(1, "S").source:sub(2)
  local script_path = vim.uv.fs_realpath(source_path) or source_path
  local editor_dir = vim.fn.fnamemodify(script_path, ":p:h:h:h:h")
  return editor_dir .. "/tree-sitter-frost"
end

local function register_frost()
  local configs = parser_configs()
  if not configs then
    return false
  end

  local existing = configs.frost or {}
  configs.frost = vim.tbl_deep_extend("force", existing, {
    install_info = {
      path = grammar_dir(),
      queries = "queries",
      generate_requires_npm = false,
      requires_generate_from_grammar = false,
    },
    filetype = "frost",
  })

  return true
end

if type(_G.frost_ts_indent) ~= "function" then
  _G.frost_ts_indent = function()
    local ok, ts_indent = pcall(require, "nvim-treesitter.indent")
    if not ok then
      return -1
    end
    return ts_indent.get_indent(vim.v.lnum)
  end
end

local function apply_frost_indent(bufnr)
  vim.bo[bufnr].autoindent = true
  vim.bo[bufnr].indentexpr = "v:lua.frost_ts_indent()"
end

local group = vim.api.nvim_create_augroup("FrostTreesitterRegister", { clear = true })
vim.api.nvim_create_autocmd("User", {
  group = group,
  pattern = { "TSUpdate", "LazyLoad", "VeryLazy" },
  callback = function(ev)
    if ev.match == "LazyLoad" and ev.data ~= "nvim-treesitter" then
      return
    end
    register_frost()
  end,
})

vim.api.nvim_create_autocmd("FileType", {
  group = group,
  pattern = "frost",
  callback = function(ev)
    apply_frost_indent(ev.buf)
  end,
})

register_frost()
if vim.bo.filetype == "frost" then
  apply_frost_indent(vim.api.nvim_get_current_buf())
end
