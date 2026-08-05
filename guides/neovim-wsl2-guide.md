# Neovim on WSL2 + zsh — A Practical Development Guide

A complete, terminal-only development setup for Python / JavaScript / TypeScript,
covering environment setup, Neovim configuration, LSPs, shortcuts, dotfiles,
git workflows, and a full software-development-lifecycle walkthrough: building
and deploying an internet-hosted CRUD app from nothing, using only WSL2, zsh,
and Neovim.

---

## Table of contents

1. [Environment setup: WSL2, zsh, Neovim](#1-environment-setup)
2. [Dotfiles and zsh aliases for an nvim workflow](#2-dotfiles-and-zsh-aliases)
3. [Neovim configuration from scratch (lazy.nvim)](#3-neovim-configuration)
4. [LSPs, formatters and linters for Python / JS / TS](#4-lsps-for-python--js--ts)
5. [Keyboard shortcuts reference](#5-keyboard-shortcuts-reference)
6. [Workflow examples](#6-workflow-examples)
7. [Git from the terminal and inside Neovim](#7-git-workflow)
8. [Practical development runbook](#8-practical-development-runbook)
9. [Full SDLC walkthrough: CRUD app from zero to deployed](#9-full-sdlc-walkthrough)
10. [Appendix: example scripts](#10-appendix-example-scripts)

---

## 1. Environment setup

### 1.1 WSL2

From an **admin PowerShell** on Windows:

```powershell
wsl --install -d Ubuntu-24.04
wsl --set-default-version 2
```

Reboot, launch Ubuntu from Windows Terminal, create your Linux user.

Two rules that will save you endless pain:

- **Keep your code inside the Linux filesystem** (`~/code/...`), *never* under
  `/mnt/c/...`. File I/O across the Windows boundary is 10–50× slower and
  breaks file-watchers (LSPs, `npm run dev`, etc.).
- Use **Windows Terminal** (or WezTerm/Alacritty) with a **Nerd Font**
  (e.g. JetBrainsMono Nerd Font) so Neovim icons render. Install the font on
  the *Windows* side and select it in the terminal profile.

Optional but recommended `/etc/wsl.conf`:

```ini
[boot]
systemd=true

[interop]
appendWindowsPath=false   # keeps Windows PATH noise out of $PATH; big speedup for zsh completion
```

Run `wsl --shutdown` from PowerShell after editing to apply.

### 1.2 zsh

```sh
sudo apt update && sudo apt install -y zsh git curl unzip ripgrep fd-find fzf build-essential
chsh -s "$(which zsh)"
```

Skip the heavyweight frameworks; three plugins give you 90% of oh-my-zsh with
none of the startup lag:

```sh
mkdir -p ~/.zsh
git clone https://github.com/zsh-users/zsh-autosuggestions ~/.zsh/zsh-autosuggestions
git clone https://github.com/zsh-users/zsh-syntax-highlighting ~/.zsh/zsh-syntax-highlighting
git clone https://github.com/sindresorhus/pure ~/.zsh/pure   # minimal async prompt
```

### 1.3 Neovim (current stable, not the apt version)

Ubuntu's apt package is ancient. Install the official tarball:

```sh
curl -LO https://github.com/neovim/neovim/releases/latest/download/nvim-linux-x86_64.tar.gz
sudo rm -rf /opt/nvim
sudo tar -C /opt -xzf nvim-linux-x86_64.tar.gz
sudo mv /opt/nvim-linux-x86_64 /opt/nvim
rm nvim-linux-x86_64.tar.gz
# /opt/nvim/bin goes on PATH in .zshrc below
```

### 1.4 Clipboard integration (WSL2 ↔ Windows)

Neovim in WSL2 can't reach the Windows clipboard by itself. Install
`win32yank`:

```sh
curl -sLo /tmp/win32yank.zip https://github.com/equalsraf/win32yank/releases/latest/download/win32yank-x64.zip
unzip -o /tmp/win32yank.zip -d /tmp/win32yank
chmod +x /tmp/win32yank/win32yank.exe
sudo mv /tmp/win32yank/win32yank.exe /usr/local/bin/
```

Neovim auto-detects `win32yank.exe` on PATH; with the config in §3 the system
clipboard just works (`"+y`, `"+p`, and the `<leader>y` mappings).

### 1.5 Language toolchains

```sh
# Node via fnm (fast, no shell lag like nvm)
curl -fsSL https://fnm.vercel.app/install | bash
fnm install --lts

# Python: uv (manages python versions, venvs, and tools)
curl -LsSf https://astral.sh/uv/install.sh | sh

# GitHub CLI — PRs from the terminal
sudo apt install -y gh

# lazygit — TUI git client that pairs beautifully with nvim
LAZYGIT_VERSION=$(curl -s https://api.github.com/repos/jesseduffield/lazygit/releases/latest | grep -Po '"tag_name": *"v\K[^"]*')
curl -Lo /tmp/lazygit.tar.gz "https://github.com/jesseduffield/lazygit/releases/download/v${LAZYGIT_VERSION}/lazygit_${LAZYGIT_VERSION}_Linux_x86_64.tar.gz"
tar -C /tmp -xzf /tmp/lazygit.tar.gz lazygit
sudo install /tmp/lazygit /usr/local/bin
```

---

## 2. Dotfiles and zsh aliases

### 2.1 The bare-repo dotfiles pattern

Track dotfiles in git without symlink managers:

```sh
git init --bare "$HOME/.dotfiles"
alias dot='git --git-dir=$HOME/.dotfiles --work-tree=$HOME'
dot config status.showUntrackedFiles no

dot add ~/.zshrc ~/.config/nvim
dot commit -m "Initial dotfiles"
gh repo create dotfiles --private --source=/dev/null || true   # or create on github.com
dot remote add origin git@github.com:YOURUSER/dotfiles.git
dot push -u origin main
```

On a new machine: `git clone --bare <url> ~/.dotfiles && dot checkout`.

### 2.2 `~/.zshrc` — a complete, nvim-centric config

```sh
# ---------- prompt & plugins ----------
fpath+=(~/.zsh/pure)
autoload -U promptinit; promptinit
prompt pure

source ~/.zsh/zsh-autosuggestions/zsh-autosuggestions.zsh
source ~/.zsh/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh

# ---------- history ----------
HISTFILE=~/.zsh_history
HISTSIZE=100000
SAVEHIST=100000
setopt SHARE_HISTORY HIST_IGNORE_ALL_DUPS HIST_REDUCE_BLANKS

# ---------- vi mode in the shell itself ----------
bindkey -v
export KEYTIMEOUT=1
bindkey '^R' history-incremental-search-backward   # keep ctrl-r in vi mode

# ---------- PATH ----------
export PATH="/opt/nvim/bin:$HOME/.local/bin:$PATH"
eval "$(fnm env --use-on-cd)"          # node
source "$HOME/.local/bin/env" 2>/dev/null   # uv

# ---------- editor everywhere ----------
export EDITOR=nvim
export VISUAL=nvim
export MANPAGER='nvim +Man!'           # read man pages in nvim

# ---------- fzf ----------
source <(fzf --zsh)                    # ctrl-r fuzzy history, ctrl-t fuzzy files
export FZF_DEFAULT_COMMAND='fd --type f --hidden --exclude .git'

# ---------- nvim-workflow aliases ----------
alias v='nvim'
alias vi='nvim'
alias vim='nvim'
alias v.='nvim .'                      # open file explorer in cwd
alias vz='nvim ~/.zshrc'               # edit shell config
alias vv='nvim ~/.config/nvim/init.lua' # edit nvim config
alias vg='nvim +Git +only'             # open straight into fugitive git status
alias vl='nvim -c "Telescope oldfiles"' # reopen recent files

# open nvim at the last file:line from ripgrep — usage: vrg "some pattern"
vrg() { nvim -q <(rg --vimgrep "$@") -c 'copen'; }

# fuzzy-pick a file and open it
vf() { local f; f=$(fzf --preview 'head -100 {}') && nvim "$f"; }

# fuzzy-cd into any project under ~/code, then open nvim
dev() { local d; d=$(fd -t d -d 2 . ~/code | fzf) && cd "$d" && nvim .; }

# ---------- git aliases ----------
alias g='git'
alias gs='git status -sb'
alias ga='git add'
alias gc='git commit'
alias gca='git commit --amend --no-edit'
alias gp='git push'
alias gpu='git push -u origin HEAD'
alias gl='git log --oneline --graph --decorate -20'
alias gd='git diff'
alias gds='git diff --staged'
alias gco='git checkout'
alias gcb='git checkout -b'
alias lg='lazygit'

# dotfiles bare repo
alias dot='git --git-dir=$HOME/.dotfiles --work-tree=$HOME'

# ---------- misc ----------
alias ll='ls -alF --color=auto'
alias fd='fdfind'                      # ubuntu names the binary fdfind
mkcd() { mkdir -p "$1" && cd "$1"; }
```

---

## 3. Neovim configuration

Everything below is a single-file config at `~/.config/nvim/init.lua`, using
**lazy.nvim** as the plugin manager. It's deliberately compact but complete —
LSP, completion, fuzzy finding, treesitter, git integration, formatting.

```lua
-- ~/.config/nvim/init.lua ------------------------------------------------

-- leader must be set before plugins load
vim.g.mapleader = " "
vim.g.maplocalleader = " "

-- ---------- options ----------
local o = vim.opt
o.number = true
o.relativenumber = true       -- 5j / 8k jumps by eye
o.signcolumn = "yes"          -- no layout shift when diagnostics appear
o.tabstop = 4
o.shiftwidth = 4
o.expandtab = true
o.smartindent = true
o.wrap = false
o.ignorecase = true
o.smartcase = true            -- case-sensitive only if pattern has capitals
o.undofile = true             -- persistent undo across sessions
o.swapfile = false
o.updatetime = 250
o.splitright = true
o.splitbelow = true
o.scrolloff = 8
o.clipboard = "unnamedplus"   -- yank/paste ↔ Windows clipboard via win32yank
o.termguicolors = true
o.completeopt = "menu,menuone,noselect"

-- ---------- bootstrap lazy.nvim ----------
local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.uv.fs_stat(lazypath) then
  vim.fn.system({ "git", "clone", "--filter=blob:none",
    "https://github.com/folke/lazy.nvim.git", "--branch=stable", lazypath })
end
vim.opt.rtp:prepend(lazypath)

require("lazy").setup({
  -- colorscheme
  { "catppuccin/nvim", name = "catppuccin", priority = 1000,
    config = function() vim.cmd.colorscheme("catppuccin-mocha") end },

  -- treesitter: proper syntax highlighting + text objects
  { "nvim-treesitter/nvim-treesitter", build = ":TSUpdate",
    config = function()
      require("nvim-treesitter.configs").setup({
        ensure_installed = { "python", "javascript", "typescript", "tsx",
          "lua", "json", "html", "css", "bash", "markdown", "sql" },
        highlight = { enable = true },
        indent = { enable = true },
      })
    end },

  -- telescope: fuzzy everything
  { "nvim-telescope/telescope.nvim", branch = "0.1.x",
    dependencies = { "nvim-lua/plenary.nvim",
      { "nvim-telescope/telescope-fzf-native.nvim", build = "make" } },
    config = function()
      local t = require("telescope")
      t.setup({})
      pcall(t.load_extension, "fzf")
    end },

  -- file explorer: edit the filesystem like a buffer
  { "stevearc/oil.nvim",
    config = function()
      require("oil").setup({ view_options = { show_hidden = true } })
    end },

  -- LSP installer + config
  { "mason-org/mason.nvim", config = true },
  { "mason-org/mason-lspconfig.nvim",
    dependencies = { "neovim/nvim-lspconfig" },
    config = function()
      require("mason-lspconfig").setup({
        ensure_installed = { "basedpyright", "ruff", "ts_ls", "eslint",
          "html", "cssls", "jsonls", "lua_ls" },
      })
    end },

  -- completion
  { "hrsh7th/nvim-cmp",
    dependencies = { "hrsh7th/cmp-nvim-lsp", "hrsh7th/cmp-buffer",
      "hrsh7th/cmp-path", "L3MON4D3/LuaSnip", "saadparwaiz1/cmp_luasnip" },
    config = function()
      local cmp = require("cmp")
      cmp.setup({
        snippet = { expand = function(args)
          require("luasnip").lsp_expand(args.body) end },
        mapping = cmp.mapping.preset.insert({
          ["<C-Space>"] = cmp.mapping.complete(),
          ["<CR>"] = cmp.mapping.confirm({ select = true }),
          ["<Tab>"] = cmp.mapping.select_next_item(),
          ["<S-Tab>"] = cmp.mapping.select_prev_item(),
        }),
        sources = cmp.config.sources(
          { { name = "nvim_lsp" }, { name = "luasnip" } },
          { { name = "buffer" }, { name = "path" } }),
      })
    end },

  -- formatting on save
  { "stevearc/conform.nvim",
    config = function()
      require("conform").setup({
        formatters_by_ft = {
          python = { "ruff_format" },
          javascript = { "prettierd", "prettier", stop_after_first = true },
          typescript = { "prettierd", "prettier", stop_after_first = true },
          html = { "prettier" }, css = { "prettier" }, json = { "prettier" },
          lua = { "stylua" },
        },
        format_on_save = { timeout_ms = 1000, lsp_format = "fallback" },
      })
    end },

  -- git
  { "lewis6991/gitsigns.nvim", config = true },  -- gutter signs, hunk ops
  { "tpope/vim-fugitive" },                       -- :Git — full git inside nvim
  { "kdheepak/lazygit.nvim",                      -- lazygit in a floating window
    dependencies = { "nvim-lua/plenary.nvim" } },

  -- quality of life
  { "windwp/nvim-autopairs", event = "InsertEnter", config = true },
  { "numToStr/Comment.nvim", config = true },     -- gcc / gc{motion}
  { "folke/which-key.nvim", event = "VeryLazy", config = true },
  { "nvim-lualine/lualine.nvim", config = true }, -- statusline
})

-- ---------- keymaps ----------
local map = vim.keymap.set

-- files & search (telescope)
local tb = require("telescope.builtin")
map("n", "<leader>ff", tb.find_files,  { desc = "Find files" })
map("n", "<leader>fg", tb.live_grep,   { desc = "Grep project" })
map("n", "<leader>fb", tb.buffers,     { desc = "Open buffers" })
map("n", "<leader>fr", tb.oldfiles,    { desc = "Recent files" })
map("n", "<leader>fw", tb.grep_string, { desc = "Grep word under cursor" })
map("n", "<leader>fd", tb.diagnostics, { desc = "Diagnostics list" })

-- file explorer
map("n", "-", "<cmd>Oil<cr>", { desc = "Parent directory (oil)" })

-- git
map("n", "<leader>gg", "<cmd>LazyGit<cr>",       { desc = "LazyGit" })
map("n", "<leader>gs", "<cmd>Git<cr>",           { desc = "Fugitive status" })
map("n", "<leader>gb", "<cmd>Git blame<cr>",     { desc = "Blame" })
map("n", "<leader>gd", "<cmd>Gvdiffsplit<cr>",   { desc = "Diff current file" })
map("n", "<leader>gh", "<cmd>Gitsigns preview_hunk<cr>", { desc = "Preview hunk" })
map("n", "<leader>gr", "<cmd>Gitsigns reset_hunk<cr>",   { desc = "Reset hunk" })
map("n", "]h", "<cmd>Gitsigns next_hunk<cr>",    { desc = "Next hunk" })
map("n", "[h", "<cmd>Gitsigns prev_hunk<cr>",    { desc = "Prev hunk" })

-- diagnostics
map("n", "[d", vim.diagnostic.goto_prev, { desc = "Prev diagnostic" })
map("n", "]d", vim.diagnostic.goto_next, { desc = "Next diagnostic" })
map("n", "<leader>e", vim.diagnostic.open_float, { desc = "Line diagnostics" })

-- LSP keymaps attach per-buffer when a server connects
vim.api.nvim_create_autocmd("LspAttach", {
  callback = function(ev)
    local opts = { buffer = ev.buf }
    map("n", "gd", tb.lsp_definitions, opts)          -- go to definition
    map("n", "gr", tb.lsp_references, opts)           -- list references
    map("n", "gI", tb.lsp_implementations, opts)
    map("n", "K",  vim.lsp.buf.hover, opts)           -- docs popup
    map("n", "<leader>rn", vim.lsp.buf.rename, opts)  -- rename symbol
    map({ "n", "v" }, "<leader>ca", vim.lsp.buf.code_action, opts)
    map("n", "<leader>ds", tb.lsp_document_symbols, opts)
  end,
})

-- terminal: <Esc> leaves terminal-insert mode
map("t", "<Esc>", [[<C-\><C-n>]])
map("n", "<leader>tt", "<cmd>botright 15split | terminal<cr>i", { desc = "Terminal" })

-- window navigation without <C-w> prefix
map("n", "<C-h>", "<C-w>h"); map("n", "<C-j>", "<C-w>j")
map("n", "<C-k>", "<C-w>k"); map("n", "<C-l>", "<C-w>l")

-- keep visual selection when indenting
map("v", "<", "<gv"); map("v", ">", ">gv")

-- move selected lines up/down
map("v", "J", ":m '>+1<CR>gv=gv"); map("v", "K", ":m '<-2<CR>gv=gv")

-- clear search highlight
map("n", "<Esc>", "<cmd>nohlsearch<cr>")

-- system clipboard explicit (even though clipboard=unnamedplus is set)
map({ "n", "v" }, "<leader>y", [["+y]], { desc = "Yank to clipboard" })
map("n", "<leader>p", [["+p]], { desc = "Paste from clipboard" })
```

First launch: `nvim` → lazy.nvim bootstraps and installs everything; Mason
downloads the language servers. Run `:checkhealth` to verify clipboard,
treesitter, and providers.

---

## 4. LSPs for Python / JS / TS

Installed automatically by Mason via the config above. What each one does:

| Server / tool    | Language | Role |
|------------------|----------|------|
| **basedpyright** | Python   | Type checking, go-to-definition, rename, imports. (Community fork of pyright; use `pyright` if you prefer upstream.) |
| **ruff**         | Python   | Lightning-fast linting **and** formatting (replaces flake8 + isort + black). Runs as an LSP for instant diagnostics + code actions (organize imports, autofix). |
| **ts_ls**        | JS/TS    | The TypeScript language server (`typescript-language-server`). Definitions, references, rename, inlay hints. For huge repos, `vtsls` is a faster alternative. |
| **eslint**       | JS/TS    | Lint rules as diagnostics with autofix code actions. Needs an `eslint.config.js` in the project. |
| **html/cssls/jsonls** | Web | Completions and validation for markup, styles, JSON (incl. `package.json`/`tsconfig.json` schemas). |
| **prettierd / prettier** | JS/TS/CSS/HTML | Formatting via conform.nvim on save. |
| **lua_ls**       | Lua      | So editing your own nvim config gets completions too. |

Per-project Python setup so basedpyright sees your venv:

```sh
cd myproject
uv venv && uv sync        # creates .venv — basedpyright auto-detects .venv/
```

Per-project TS: a `tsconfig.json` at the root is all `ts_ls` needs.

---

## 5. Keyboard shortcuts reference

Leader is **Space**. Shortcuts marked (core) are stock Vim/Neovim; the rest
come from the config in §3.

### Movement (core)

| Keys | Action |
|------|--------|
| `h j k l` | left / down / up / right |
| `w` / `b` / `e` | next word / back word / end of word |
| `0` / `^` / `$` | line start / first char / line end |
| `gg` / `G` | top / bottom of file |
| `{` / `}` | previous / next paragraph or block |
| `%` | jump to matching bracket |
| `f{char}` / `t{char}` | jump onto / just before char in line (`;` repeats) |
| `Ctrl-d` / `Ctrl-u` | half-page down / up |
| `Ctrl-o` / `Ctrl-i` | jump back / forward in jump history |
| `*` | search word under cursor |
| `5j`, `12k` | relative-number jumps |

### Editing (core)

| Keys | Action |
|------|--------|
| `i` / `a` / `o` / `O` | insert before / after / line below / line above |
| `ciw` | change inner word |
| `ci"` `ci(` `ci{` | change inside quotes / parens / braces |
| `daw` / `dap` | delete a word / a paragraph |
| `dd` / `yy` / `p` | delete line / yank line / paste |
| `.` | repeat last change (the most underrated key in Vim) |
| `u` / `Ctrl-r` | undo / redo |
| `>` `<` (visual) | indent / dedent, selection kept |
| `J` / `K` (visual) | move selected lines down / up |
| `gcc` / `gc{motion}` | toggle comment (Comment.nvim) |
| `Ctrl-v` | visual block — column edits, multi-line insert with `I`/`A` |
| `:%s/old/new/gc` | project-file find & replace with confirm |

### Files & search (Telescope / Oil)

| Keys | Action |
|------|--------|
| `Space ff` | fuzzy find files |
| `Space fg` | live grep the whole project |
| `Space fw` | grep word under cursor |
| `Space fb` | switch between open buffers |
| `Space fr` | recent files |
| `Space fd` | all diagnostics |
| `-` | open parent directory as an editable buffer (Oil); edit names, `:w` renames files |

### LSP

| Keys | Action |
|------|--------|
| `gd` | go to definition |
| `gr` | list all references |
| `gI` | go to implementation |
| `K` | hover documentation |
| `Space rn` | rename symbol project-wide |
| `Space ca` | code actions (auto-import, fix, organize imports) |
| `Space ds` | document symbols (outline) |
| `[d` / `]d` | previous / next diagnostic |
| `Space e` | show diagnostic under cursor |

### Git

| Keys | Action |
|------|--------|
| `Space gg` | LazyGit floating window |
| `Space gs` | fugitive `:Git` status (stage with `s`, commit with `cc`) |
| `Space gd` | side-by-side diff of current file |
| `Space gb` | git blame |
| `]h` / `[h` | next / previous changed hunk |
| `Space gh` | preview hunk diff |
| `Space gr` | reset (revert) hunk |

### Windows, splits, terminal

| Keys | Action |
|------|--------|
| `Ctrl-h/j/k/l` | move between splits |
| `:vsp` / `:sp` | vertical / horizontal split |
| `Space tt` | open terminal in bottom split |
| `Esc` (in terminal) | back to normal mode |
| `Ctrl-w =` | equalize split sizes |
| `Ctrl-w o` | close all other windows |

---

## 6. Workflow examples

### 6.1 "Find where this function is used and change its signature"

1. Cursor on the function name → `gr` → Telescope lists every reference.
2. `Ctrl-q` in Telescope sends the results to the quickfix list.
3. `Space rn` → type new name → LSP renames across the project.
4. `]d` to hop through any resulting type errors and fix them.

### 6.2 "Explore an unfamiliar codebase"

1. `nvim .` → Oil shows the tree; drill in with `Enter`, up with `-`.
2. `Space fg` → grep for a domain term ("invoice", "session").
3. On something interesting: `gd` to its definition, `Ctrl-o` to come back.
4. `Space ds` for a symbol outline of a big file.

### 6.3 "Edit-run loop for a script"

1. `Space tt` opens a terminal split; run `uv run python app.py` or `npm run dev`.
2. `Ctrl-k` back to the code, edit, `:w` (auto-formats via conform).
3. `Ctrl-j` into the terminal, `↑ Enter` to rerun. Servers with reload
   (`uvicorn --reload`, `vite`) restart on save automatically.

### 6.4 "Bulk edit from grep results"

```sh
vrg 'TODO\(alice\)'      # zsh function from §2: opens nvim with quickfix loaded
```

Then in nvim: `:cfdo %s/TODO(alice)/TODO(bob)/g | update` edits every match
in every file.

### 6.5 "Column edit" (visual block)

Add a prefix to 20 lines: `Ctrl-v`, `19j`, `I`, type prefix, `Esc` — applied to
all lines at once.

---

## 7. Git workflow

Three layers, from lightest to heaviest:

1. **gitsigns** (ambient): gutter markers, `]h`/`[h` to jump hunks,
   `Space gh` to peek a diff, `Space gr` to discard a hunk. Great for
   reviewing your own edits before staging.
2. **fugitive** (`Space gs`): interactive status buffer. `s` stages the
   file/hunk under cursor, `u` unstages, `=` toggles inline diff, `cc` opens
   the commit message in a split, `dv` opens a vertical diff.
   `:Git log --oneline`, `:Git push` — anything git can do.
3. **lazygit** (`Space gg`): full TUI for branch juggling, interactive
   rebases, cherry-picks, stash management.

And `gh` for everything GitHub-side — creating repos, PRs, reviews, merges —
demonstrated end-to-end in §9.

Commit hygiene that pays off: small commits, imperative subject lines
("Add task deletion endpoint"), one logical change per commit. `gca`
(commit --amend --no-edit) for "oops, forgot a file" moments before pushing.

---

## 8. Practical development runbook

Day-to-day sequence for any feature, entirely in the terminal:

```text
1.  dev                          # zsh function: fuzzy-pick project, cd, open nvim
2.  Space gg → pull latest       # or: git pull origin main
3.  gcb feat/my-feature          # new branch (alias for git checkout -b)
4.  Code:
      Space ff / Space fg        # navigate
      gd / gr / K                # understand
      edit, :w                   # format-on-save runs ruff/prettier
      ]d, Space ca               # fix diagnostics as they appear
5.  Test in a terminal split (Space tt):
      uv run pytest -x           # or: npm test
6.  Review your own diff:        Space gs, walk files with = toggled
7.  Stage & commit:              s on each file in fugitive, cc, write message
8.  gpu                          # git push -u origin HEAD
9.  gh pr create --fill          # PR from the terminal
10. gh pr checks --watch         # wait for CI
11. gh pr merge --squash --delete-branch
12. gco main && git pull         # back to a clean main
```

When something breaks in production or CI:

```text
gh run list --limit 5            # what failed?
gh run view <id> --log-failed    # read the failing step's log
git log --oneline -10            # what changed recently?
git bisect start / good / bad    # if the culprit isn't obvious
```

---

## 9. Full SDLC walkthrough

**Goal:** a task-manager CRUD app — Python + SQLite backend, vanilla
TypeScript frontend — hosted on the public internet for a few concurrent
users. Built from nothing in WSL2/zsh with Neovim as the only editor, with
git init → branches → PRs → merge → deploy all from the terminal.

Stack choices, sized to "a few concurrent users":

- **Backend:** FastAPI + uvicorn (single process is plenty), `sqlite3` from
  the standard library — no ORM.
- **Frontend:** one HTML file, one vanilla TS file compiled with `tsc`,
  no framework, no bundler.
- **Hosting:** a $4–6/mo VPS (Hetzner/DigitalOcean) with systemd + **Caddy**
  (automatic HTTPS). SQLite lives on the VPS disk — exactly right for this
  scale.

### 9.1 Scaffold and first commit

```sh
mkcd ~/code/taskman
git init -b main
uv init --no-workspace && uv add "fastapi[standard]"
npm init -y && npm install -D typescript
mkdir -p app static/src

nvim .gitignore
```

`.gitignore`:

```gitignore
.venv/
__pycache__/
node_modules/
static/dist/
*.db
```

```sh
git add -A && git commit -m "Scaffold project: uv + fastapi, npm + tsc"

# create the GitHub repo and push — no browser needed
gh auth login          # one-time; choose SSH
gh repo create taskman --private --source=. --push
```

### 9.2 Database layer — `nvim app/db.py`

```python
import sqlite3
from contextlib import contextmanager
from pathlib import Path

DB_PATH = Path(__file__).parent.parent / "taskman.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"""


@contextmanager
def get_db():
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")  # readers don't block the writer
    conn.execute("PRAGMA busy_timeout=5000")  # wait instead of 'database is locked'
    try:
        yield conn
        conn.commit()
    except Exception:
        conn.rollback()
        raise
    finally:
        conn.close()


def init_db() -> None:
    with get_db() as conn:
        conn.executescript(SCHEMA)
```

The two PRAGMAs are the whole "few concurrent users on SQLite" story: WAL mode
lets reads and writes overlap, and `busy_timeout` makes writers queue politely
instead of erroring.

### 9.3 API — `nvim app/main.py`

```python
from fastapi import FastAPI, HTTPException
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

from app.db import get_db, init_db

app = FastAPI(title="taskman")
init_db()


class TaskIn(BaseModel):
    title: str


class TaskPatch(BaseModel):
    title: str | None = None
    done: bool | None = None


@app.get("/api/tasks")
def list_tasks():
    with get_db() as conn:
        rows = conn.execute("SELECT * FROM tasks ORDER BY id DESC").fetchall()
    return [dict(r) for r in rows]


@app.post("/api/tasks", status_code=201)
def create_task(task: TaskIn):
    with get_db() as conn:
        cur = conn.execute("INSERT INTO tasks (title) VALUES (?)", (task.title,))
        row = conn.execute(
            "SELECT * FROM tasks WHERE id = ?", (cur.lastrowid,)
        ).fetchone()
    return dict(row)


@app.patch("/api/tasks/{task_id}")
def update_task(task_id: int, patch: TaskPatch):
    fields, values = [], []
    if patch.title is not None:
        fields.append("title = ?"); values.append(patch.title)
    if patch.done is not None:
        fields.append("done = ?"); values.append(int(patch.done))
    if not fields:
        raise HTTPException(400, "nothing to update")
    with get_db() as conn:
        cur = conn.execute(
            f"UPDATE tasks SET {', '.join(fields)} WHERE id = ?",
            (*values, task_id),
        )
        if cur.rowcount == 0:
            raise HTTPException(404, "task not found")
        row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()
    return dict(row)


@app.delete("/api/tasks/{task_id}", status_code=204)
def delete_task(task_id: int):
    with get_db() as conn:
        cur = conn.execute("DELETE FROM tasks WHERE id = ?", (task_id,))
        if cur.rowcount == 0:
            raise HTTPException(404, "task not found")


# serve the frontend; mounted last so /api/* wins
app.mount("/", StaticFiles(directory="static", html=True), name="static")
```

While typing this, the nvim setup earns its keep: basedpyright flags typos in
field names instantly, `Space ca` auto-imports `HTTPException`, ruff formats
on `:w`, and `K` on `StaticFiles` shows its docs.

### 9.4 Frontend — `nvim static/index.html`

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>taskman</title>
  <style>
    body { font: 16px/1.5 system-ui; max-width: 40rem; margin: 3rem auto; padding: 0 1rem; }
    form { display: flex; gap: .5rem; margin-bottom: 1.5rem; }
    input[type=text] { flex: 1; padding: .5rem; }
    li { display: flex; gap: .5rem; align-items: center; padding: .25rem 0; }
    li.done span { text-decoration: line-through; opacity: .5; }
    ul { list-style: none; padding: 0; }
    button.del { margin-left: auto; }
  </style>
</head>
<body>
  <h1>taskman</h1>
  <form id="new-task">
    <input type="text" id="title" placeholder="What needs doing?" required />
    <button>Add</button>
  </form>
  <ul id="tasks"></ul>
  <script type="module" src="/dist/app.js"></script>
</body>
</html>
```

### 9.5 `nvim static/src/app.ts`

```typescript
interface Task {
  id: number;
  title: string;
  done: number;
  created_at: string;
}

const list = document.querySelector<HTMLUListElement>("#tasks")!;
const form = document.querySelector<HTMLFormElement>("#new-task")!;
const titleInput = document.querySelector<HTMLInputElement>("#title")!;

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api${path}`, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  if (!res.ok) throw new Error(`${res.status} ${await res.text()}`);
  return res.status === 204 ? (undefined as T) : res.json();
}

function render(tasks: Task[]): void {
  list.replaceChildren(
    ...tasks.map((t) => {
      const li = document.createElement("li");
      li.className = t.done ? "done" : "";

      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = !!t.done;
      checkbox.onchange = async () => {
        await api(`/tasks/${t.id}`, {
          method: "PATCH",
          body: JSON.stringify({ done: checkbox.checked }),
        });
        refresh();
      };

      const span = document.createElement("span");
      span.textContent = t.title;

      const del = document.createElement("button");
      del.className = "del";
      del.textContent = "✕";
      del.onclick = async () => {
        await api(`/tasks/${t.id}`, { method: "DELETE" });
        refresh();
      };

      li.append(checkbox, span, del);
      return li;
    }),
  );
}

async function refresh(): Promise<void> {
  render(await api<Task[]>("/tasks"));
}

form.onsubmit = async (e) => {
  e.preventDefault();
  await api("/tasks", {
    method: "POST",
    body: JSON.stringify({ title: titleInput.value.trim() }),
  });
  form.reset();
  refresh();
};

refresh();
```

`nvim tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "es2022",
    "module": "es2022",
    "strict": true,
    "outDir": "static/dist",
    "rootDir": "static/src"
  },
  "include": ["static/src"]
}
```

### 9.6 Run it locally

Inside nvim, `Space tt` for a terminal split (or a second Windows Terminal
tab):

```sh
npx tsc --watch &                 # recompiles app.ts on save
uv run fastapi dev app/main.py    # http://localhost:8000, auto-reload
```

Open `http://localhost:8000` in your Windows browser — WSL2 forwards
localhost automatically. Add, toggle, delete tasks; watch requests in the
uvicorn log.

Quick API smoke tests without leaving the terminal:

```sh
curl -s localhost:8000/api/tasks | python3 -m json.tool
curl -s -X POST localhost:8000/api/tasks -H 'content-type: application/json' -d '{"title":"ship it"}'
```

### 9.7 Tests, then the feature-branch → PR → merge loop

```sh
uv add --dev pytest httpx
gcb feat/tests                     # git checkout -b feat/tests
nvim tests/test_api.py
```

```python
from fastapi.testclient import TestClient

from app.main import app

client = TestClient(app)


def test_crud_roundtrip():
    created = client.post("/api/tasks", json={"title": "write tests"}).json()
    assert created["title"] == "write tests"

    task_id = created["id"]
    patched = client.patch(f"/api/tasks/{task_id}", json={"done": True}).json()
    assert patched["done"] == 1

    assert client.delete(f"/api/tasks/{task_id}").status_code == 204
    assert client.patch(f"/api/tasks/{task_id}", json={"done": False}).status_code == 404
```

```sh
uv run pytest -x                   # green?
```

Now the PR cycle, 100% terminal:

```sh
# review the diff inside nvim: Space gs, stage with s, commit with cc
git add -A
git commit -m "Add API tests covering the CRUD roundtrip"
gpu                                # git push -u origin HEAD
gh pr create --title "Add API tests" --body "Covers create/patch/delete/404 paths."
gh pr view --web                   # optional: peek in browser
gh pr merge --squash --delete-branch
gco main && git pull origin main
```

Add CI so every future PR runs the tests — `nvim .github/workflows/ci.yml`:

```yaml
name: ci
on:
  pull_request:
  push:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v5
      - run: uv sync
      - run: uv run pytest
      - run: npx --yes tsc --noEmit || npm ci && npx tsc --noEmit
```

Commit it on a branch, `gh pr create --fill`, `gh pr checks --watch`, merge.
From now on `gh pr checks` is step 10 of every runbook cycle.

### 9.8 Deploy to the internet

Provision the cheapest VPS at Hetzner/DigitalOcean (Ubuntu 24.04), point a
DNS A-record (`taskman.example.com`) at its IP, then from WSL2:

```sh
ssh root@taskman.example.com
```

On the server:

```sh
adduser deploy && usermod -aG sudo deploy
rsync --archive --chown=deploy:deploy ~/.ssh /home/deploy   # copy your key
apt update && apt install -y caddy
curl -LsSf https://astral.sh/uv/install.sh | sh   # as the deploy user

# get the code
su - deploy
git clone https://github.com/YOURUSER/taskman.git ~/taskman
cd ~/taskman && uv sync && npx --yes tsc
```

`sudo nvim /etc/systemd/system/taskman.service`:

```ini
[Unit]
Description=taskman
After=network.target

[Service]
User=deploy
WorkingDirectory=/home/deploy/taskman
ExecStart=/home/deploy/.local/bin/uv run uvicorn app.main:app --host 127.0.0.1 --port 8000
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

`sudo nvim /etc/caddy/Caddyfile`:

```caddyfile
taskman.example.com {
    reverse_proxy 127.0.0.1:8000
}
```

```sh
sudo systemctl enable --now taskman
sudo systemctl reload caddy
```

Caddy fetches a Let's Encrypt certificate automatically — the app is now live
at `https://taskman.example.com` with HTTPS, surviving reboots, easily
handling a few concurrent users on one uvicorn process.

Deploying a new version after merging a PR — `nvim deploy.sh` in the repo:

```sh
#!/usr/bin/env zsh
set -euo pipefail
ssh deploy@taskman.example.com 'cd ~/taskman &&
  git pull origin main &&
  uv sync &&
  npx tsc &&
  sudo systemctl restart taskman'
echo "deployed $(git rev-parse --short origin/main)"
```

```sh
chmod +x deploy.sh
./deploy.sh
```

Backups (SQLite makes this trivial) — cron on the server:

```sh
crontab -e   # opens in nvim, since EDITOR=nvim
# nightly consistent snapshot even while the app is writing:
0 3 * * * sqlite3 ~/taskman/taskman.db ".backup ~/backups/taskman-$(date +\%F).db"
```

### 9.9 The lifecycle, complete

Every phase happened in the terminal with nvim as the only editor:

| Phase | Tools used |
|-------|-----------|
| Scaffold | `uv init`, `npm init`, nvim, `git init`, `gh repo create --push` |
| Develop | nvim + LSPs (basedpyright, ruff, ts_ls), format-on-save, terminal split |
| Test | pytest + httpx in a split; `curl` smoke tests |
| Review | gitsigns hunks, fugitive diffs, `gh pr create` / `gh pr checks` |
| Integrate | GitHub Actions CI, `gh pr merge --squash` |
| Deploy | ssh, systemd, Caddy (auto-HTTPS), `deploy.sh` |
| Operate | `systemctl status`, `journalctl -u taskman -f`, sqlite `.backup` cron |

Rinse and repeat from step 3 of the runbook (§8) for every feature after.

---

## 10. Appendix: example scripts

Small scripts worth writing early — they compound.

### 10.1 `~/bin/proj-new` — new project scaffolder

```sh
#!/usr/bin/env zsh
# usage: proj-new myapp [python|ts]
set -euo pipefail
name=$1 kind=${2:-python}
mkdir -p ~/code/$name && cd ~/code/$name
git init -b main
case $kind in
  python) uv init --no-workspace && uv add --dev pytest ruff ;;
  ts)     npm init -y && npm i -D typescript && npx tsc --init ;;
esac
print '.venv/\n__pycache__/\nnode_modules/\ndist/\n*.db' > .gitignore
git add -A && git commit -m "Scaffold $kind project"
nvim .
```

### 10.2 `~/bin/git-wip` — checkpoint everything fast

```sh
#!/usr/bin/env zsh
git add -A && git commit -m "wip: $(date +%H:%M) ${*:-checkpoint}"
```

### 10.3 `~/bin/serve-here` — throwaway static server for any directory

```sh
#!/usr/bin/env zsh
python3 -m http.server "${1:-8080}"
```

### 10.4 fzf-powered branch switcher (drop in `.zshrc`)

```sh
gsw() {
  local b
  b=$(git branch --all --format='%(refname:short)' | grep -v HEAD | fzf) || return
  git switch "${b#origin/}"
}
```

Make them executable and on PATH:

```sh
mkdir -p ~/bin && chmod +x ~/bin/*
# in .zshrc: export PATH="$HOME/bin:$PATH"
```

---

*Everything in this guide — the shell, the editor, git, GitHub, tests,
deployment, and operations — runs in one terminal window. The payoff of the
setup cost is a workflow where navigating, editing, reviewing, and shipping
are all the same keystrokes, on any machine you can SSH into.*
