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

register_frost()
