-- Rustor LSP configuration for Neovim
-- Add this to your Neovim config or use with nvim-lspconfig

local M = {}

-- Default configuration
M.config = {
  cmd = { "rustor", "--lsp" },
  filetypes = { "php" },
  root_dir = function(fname)
    return vim.fn.getcwd()
  end,
  settings = {},
}

-- Setup function for manual configuration
function M.setup(opts)
  opts = opts or {}

  local config = vim.tbl_deep_extend("force", M.config, opts)

  vim.api.nvim_create_autocmd("FileType", {
    pattern = "php",
    callback = function()
      vim.lsp.start(config)
    end,
  })
end

-- For use with nvim-lspconfig
-- Add to your lspconfig setup:
--[[
  require('lspconfig.configs').rustor = {
    default_config = {
      cmd = { 'rustor', '--lsp' },
      filetypes = { 'php' },
      root_dir = function(fname)
        return vim.fn.getcwd()
      end,
    },
  }
  require('lspconfig').rustor.setup{}
]]

return M
