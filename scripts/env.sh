# Sourced by the other scripts. Makes `node`/`npm` resolvable in non-interactive
# shells on machines where they only exist behind a lazy nvm shell function.
unset -f node npm npx nvm 2>/dev/null || true
if ! command -v node >/dev/null 2>&1 && [ -d "$HOME/.nvm/versions/node" ]; then
  latest="$(ls "$HOME/.nvm/versions/node" | sort -V | tail -1)"
  export PATH="$HOME/.nvm/versions/node/$latest/bin:$PATH"
fi
